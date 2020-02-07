#[macro_use] extern crate log;
extern crate async_std;
//extern crate crossbeam_deque;
//use maxminddb::geoip2;

mod traceroute;

use std::thread;
use std::collections::HashMap;
use std::net::Ipv4Addr;

use lib_data::{AppData, ReceiverChannel, AppTcp, AppTraceRoute, AppTraceRouteTask, OptConf};
use redis::Commands;

pub fn start(rx:ReceiverChannel, ip: std::net::Ipv4Addr, opts:& OptConf){
    info!("Starting tracer loop...");


    let redis_url = opts.redis_url.clone().unwrap_or_else(||"redis://localhost/net-gazer".into());
    info!("About to attempt to connect to '{}'...", redis_url);

    thread::spawn(move || {


        //TODO: eviction policy
        // some local net ip  and remote ip
        let mut tcp_map = HashMap::<Ipv4Addr, AppTcp>::new();

        //TODO: eviction policy
        //dst as key
        let mut tr_map = HashMap::<Ipv4Addr, AppTraceRoute>::new();

        let mut builder = lib_fbuffers::Builder::default();

        let mut  id_seq = 0u16; //0-65535

        let mut conn = 
            match redis::Client::open(redis_url)
                .and_then(|client| client.get_connection()){
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Redis connectivity failed! Error:{}",e);
                        std::process::exit(-1);
                    }
            };
    
        loop{
            if let Ok(msg) = rx.recv(){
                match msg.clone() {
                    AppData::Syn(mut m) => { //outbound, use dst
                        if let Some(tcp) = tcp_map.get_mut(m.get_key()){
                            tcp.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);
                            tcp_map.insert(*m.get_key(), m.clone())
                                .and_then::<Option<()>, _>(|_| {error!("syn tcp dup:{}", m.get_key());None});

                            let trace = AppTraceRoute::new(ip, m.dst, m.id);
                            let msg = builder.create_route_message(&[trace.clone()]);
                            redis::cmd("LPUSH")
                                .arg(&*m.get_key().to_string())
                                //.arg(msg.clone())
                                .arg(&*m.get_key().to_string())
                                .query::<()>(&mut conn).unwrap();



                            tr_map.insert(trace.get_key(), trace)
                                .and_then::<Option<()>, _>(|_| {warn!("syn trace dup:{}", m.dst); None});

                            debug!("SYN    : {}", m); //,to_string(city));

                            tr_map.get_mut(&m.get_key())
                                .and_then::<Option<()>, _>(|trace|{
                                    trace.request = Some(AppTraceRouteTask::from(&*trace));
                                    traceroute::process(trace.request.clone().unwrap());
                                    None
                                });
                        }
                    }
                    AppData::SynAck(mut m) => { //inbound, use src
                        if let Some(d) = tcp_map.get_mut(m.get_key()){
                            d.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);
                            tcp_map.insert(*m.get_key(), m.clone())
                                .and_then::<Option<()>, _>(|_| {error!("synack tcp dup:{}", m.get_key());None});

                            let trace = AppTraceRoute::new(ip, m.dst, m.id);
                            let msg = builder.create_route_message(&[trace.clone()]);
                            redis::cmd("LPUSH")
                                .arg(&*m.get_key().to_string())
                                //.arg(msg.clone())
                                .arg(&*m.get_key().to_string())
                                .query::<()>(&mut conn).unwrap();

                            tr_map.insert(trace.get_key(), trace)
                                .and_then::<Option<()>, _>(|_| {warn!("synack trace dup:{}", m.dst); None});

                            debug!("SYN-ACK: {}", m);

                            tr_map.get_mut(&m.get_key())
                                .and_then::<Option<()>, _>(|trace|{
                                    trace.request = Some(AppTraceRouteTask::from(&*trace));
                                    traceroute::process(trace.request.clone().unwrap());
                                    None
                                });
                        }
                    }
                    AppData::IcmpReply(m) => {
                        trace!("ICMP-Reply: {}", m);

                        tr_map.get_mut(&m.get_key())
                            .and_then(|trace|{
                                let ret = trace.add_trace(&msg);
                                if let Some(req) = trace.request.clone(){ 
                                    traceroute::process(req);
                                }
                                ret
                            })                                    
                            .and_then::<Option<()>, _>(|hop|{
                                let msg = builder.create_hop_message(&[hop]);
                                redis::cmd("LPUSH")
                                    .arg(&*m.get_key().to_string())
                                    //.arg(msg.clone())
                                    .arg(&*m.hop.to_string())
                                    .query::<()>(&mut conn).unwrap();


                                info!("icmp-reply[compl'd]:{}\t{}->{}->\t{}, distance:{} ",m.pkt_id, m.src, m.hop, m.dst, m.pkt_seq);
                                None
                            });
                    }
                    AppData::IcmpExceeded(m) => {
                        trace!("ICMP-Exceeded: {}", m);

                        tr_map.get_mut(&m.get_key())
                            .and_then(|trace|{
                                let ret = trace.add_trace(&msg);
                                if let Some(req) = trace.request.clone(){
                                    traceroute::process(req);
                                }
                                ret
                            })
                            .and_then::<Option<()>, _>(|hop|{
                                let msg = builder.create_hop_message(&[hop]);
                                redis::cmd("LPUSH")
                                    .arg(&*m.get_key().to_string())
                                    //.arg(msg.clone())
                                    .arg(&*m.hop.to_string())
                                    .query::<()>(&mut conn).unwrap();


                                info!("icmp-exeeded:{}\t{}->[{}]{}->\t{}", m.pkt_id, m.src, m.pkt_seq, m.hop, m.dst);
                                None
                            });
                    }
                    AppData::IcmpUnreachable(m) => {
                        trace!("ICMP-Unreachable: {}", m);

                        tr_map.get_mut(&m.get_key())
                            .and_then(|trace|trace.add_trace(&msg))
                            .and_then::<Option<()>, _>(|hop|{
                                let msg = builder.create_hop_message(&[hop]);
                                redis::cmd("LPUSH")
                                    .arg(&*m.get_key().to_string())
                                    //.arg(msg.clone())
                                    .arg(&*m.hop.to_string())
                                    .query::<()>(&mut conn).unwrap();

                                info!("icmp-unreachable:{}\t{}->[{}]{}->\t{}",m.pkt_id, m.src,m.ttl,m.hop, m.dst);
                                None
                            });
                    }
                    AppData::Timer(_now) =>{
                        //cleanup...
                        tr_map.values_mut()
                            .filter(|tr| tr.request.is_some())
                            .filter(|tr| {
                                if let Some(task) = tr.request.clone(){
                                    return task.ts.elapsed().as_secs() > 5
                                }
                                false
                            }).for_each(|tr| {
                                info!("timer: id:{} {} ttl:{}, hops:{}, missing:{} ",tr.pkt_id, tr.dst, tr.ttl, tr.trace.len(), tr.ttl - tr.trace.len() as u8);
                                tr.setup_for_next_request();
                                if let Some(req) = tr.request.clone(){ 
                                    traceroute::process(req);
                                }
                            });
                    }

                }
            }
        }

    });
}

fn increment(seq:u16) -> u16{
    if let Some(v) = seq.checked_add(1){
        v
    }else{
        0
    }
}


pub fn timer_start(tx:lib_data::SenderChannel){
    info!("Starting timer loop...");

    thread::spawn(move || {
        //maintenance period... arbitrary 5 sec
        let sleep_duration = std::time::Duration::new(5,0);

        loop{
            thread::sleep(sleep_duration);
            trace!("Timer event");
            tx.send(AppData::Timer(std::time::Instant::now())).unwrap();
        }
    });



}

// fn to_string(d:Option<geoip2::City>) ->String{
//     if d.is_none() {
//         return "Unknown".into();
//     }


//     let city = d.unwrap();

//     let mut ret_val = String::new();

//     if let Some(city) = city.city{
//         if let Some(city_name) = city.names.unwrap().get("en"){
//             ret_val.push_str(&city_name);
//         }else{
//             ret_val.push_str("N/A");
//         }
//     }else{
//         ret_val.push_str("N/A");
//     }

//     ret_val.push_str(", ");

//     if let Some(country) = city.country{
//         if let Some(country_name) = country.names.unwrap().get("en"){
//             ret_val.push_str(&country_name);
//         }else{
//             ret_val.push_str("N/A");
//         }
//     }else{
//         ret_val.push_str("N/A");
//     }

//     ret_val
// }