#[macro_use] extern crate log;
extern crate async_std;

mod traceroute;
mod consume;

use std::thread;
use std::collections::HashMap;
use std::net::Ipv4Addr;

use lib_data::{AppData, ReceiverChannel, AppTcp, AppTraceRoute, AppTraceRouteTask, OptConf};

pub fn start(rx:ReceiverChannel, ip: std::net::Ipv4Addr, opts:& OptConf){
    info!("Starting tracer loop...");

    let reporting_url = opts.reporting_url.clone().unwrap();

    thread::spawn(move || {


        //TODO: eviction policy
        // some local net ip  and remote ip
        let mut tcp_map = HashMap::<Ipv4Addr, AppTcp>::new();

        //TODO: eviction policy
        //dst as key
        let mut tr_map = HashMap::<Ipv4Addr, AppTraceRoute>::new();

        let mut builder = lib_fbuffers::Builder::default();

        let mut  id_seq = 0u16; //0-65535
    

        loop{
            if let Ok(msg) = rx.recv(){
                match msg.clone() {
                    AppData::Syn(mut m) => { //outbound, use dst
                        if let Some(tcp) = tcp_map.get_mut(m.get_key()){
                            tcp.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);

                            debug!("SYN    : {}", m);

                            //communications map
                            tcp_map.insert(*m.get_key(), m.clone())
                                .and_then::<Option<()>, _>(|_| {error!("syn tcp dup:{}", m.get_key());None});

                            let mut trace = AppTraceRoute::new(ip, m.dst, m.id);
                            trace.request = Some(AppTraceRouteTask::from(&trace));
                            
                            //traces map
                            tr_map.insert(trace.get_key(), trace.clone())
                                .and_then::<Option<()>, _>(|_| {warn!("syn trace dup:{}", m.dst); None});

                            consume::consume_route(&mut builder, &trace, &reporting_url);

                        }
                    }
                    AppData::SynAck(mut m) => { //inbound, use src
                        if let Some(d) = tcp_map.get_mut(m.get_key()){
                            d.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);

                            debug!("SYN-ACK: {}", m);

                            tcp_map.insert(*m.get_key(), m.clone())
                                .and_then::<Option<()>, _>(|_| {error!("synack tcp dup:{}", m.get_key());None});

                            let mut trace = AppTraceRoute::new(ip, m.dst, m.id);
                            trace.request = Some(AppTraceRouteTask::from(&trace));

                            tr_map.insert(trace.get_key(), trace.clone())
                                .and_then::<Option<()>, _>(|_| {warn!("synack trace dup:{}", m.dst); None});

                            consume::consume_route(&mut builder, &trace, &reporting_url);
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
                                consume::consume_hop(&mut builder, &hop, &reporting_url);
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
                                consume::consume_hop(&mut builder, &hop, &reporting_url);
                                info!("icmp-exeeded:{}\t{}->[{}]{}->\t{}", m.pkt_id, m.src, m.pkt_seq, m.hop, m.dst);
                                None
                            });
                    }
                    AppData::IcmpUnreachable(m) => {
                        trace!("ICMP-Unreachable: {}", m);

                        tr_map.get_mut(&m.get_key())
                            .and_then(|trace|trace.add_trace(&msg))
                            .and_then::<Option<()>, _>(|hop|{
                                consume::consume_hop(&mut builder, &hop, &reporting_url);
                                info!("icmp-unreachable:{}\t{}->[{}]{}->\t{}",m.pkt_id, m.src,m.ttl,m.hop, m.dst);
                                None
                            });
                    }
                    AppData::Timer(_now) =>{
                        //FIXME: add cleanup...

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