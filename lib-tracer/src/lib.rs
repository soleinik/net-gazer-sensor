#[macro_use] extern crate log;
extern crate async_std;
//extern crate crossbeam_deque;
//use maxminddb::geoip2;

mod traceroute;

use std::thread;
use std::collections::HashMap;
use std::net::Ipv4Addr;

use lib_data::{AppData, ReceiverChannel, AppTcp, AppTraceRoute, AppTraceRouteTask};


pub fn start(rx:ReceiverChannel, ip: std::net::Ipv4Addr){
    info!("Starting tracer loop...");
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
                            tcp_map.insert(*m.get_key(), m.clone());

                            let trace = AppTraceRoute::new(ip, m.dst, m.id);
                            let msg = builder.create_route_message(&[trace.clone()]);
                            println!("syn:{:?}", msg);

                            let dup = tr_map.insert(trace.get_key(), trace);
                            if dup.is_some(){
                                warn!("syn dup:{}", m.dst);
                            }

                            debug!("SYN    : {}", m); //,to_string(city));

                            /* submit for trace - fire and forget... */
                            if let Some(trace) = tr_map.get_mut(&m.get_key()){
                                trace.request = Some(AppTraceRouteTask::from(&*trace));
                                traceroute::process(trace.request.clone().unwrap());
                            }
                        }
                    }
                    AppData::SynAck(mut m) => { //inbound, use src
                        if let Some(d) = tcp_map.get_mut(m.get_key()){
                            d.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);
                            tcp_map.insert(*m.get_key(), m.clone());

                            let trace = AppTraceRoute::new(ip, m.dst, m.id);
                            let msg = builder.create_route_message(&[trace.clone()]);
                            println!("syn-ack:{:?}", msg);

                            let dup = tr_map.insert(trace.get_key(), trace);
                            if dup.is_some(){
                                warn!("synack dup:{}", m.dst);
                            }


                            //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
                            debug!("SYN-ACK: {}", m); //,to_string(city));

                            /* submit for trace - fire and forget... */
                            if let Some(trace) = tr_map.get_mut(&m.get_key()){
                                trace.request = Some(AppTraceRouteTask::from(&*trace));
                                traceroute::process(trace.request.clone().unwrap());
                            }
                        }
                    }
                    AppData::IcmpReply(m) => {
                        trace!("ICMP-Reply: {}", m);

                        if let Some(trace) = tr_map.get_mut(&m.get_key()){

                            if let Some(hop) = trace.add_trace(&msg){
                                let msg = builder.create_hop_message(&[hop]);
                                println!("icmp-reply:{}\t{}->[{}]{}->\t{}",m.pkt_id, m.src, m.ttl, m.hop, m.dst);
                            }

                            if let Some(req) = trace.request.clone(){ 
                                traceroute::process(req);
                            }
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::IcmpExceeded(m) => {
                        trace!("ICMP-Exceeded: {}", m);

                        if let Some(trace) = tr_map.get_mut(&m.get_key()){

                            if let Some(hop) = trace.add_trace(&msg){
                                let msg = builder.create_hop_message(&[hop]);
                                println!("icmp-exeeded:{}\t{}->[{}]{}->\t{}", m.pkt_id, m.src, m.pkt_seq, m.hop, m.dst);
                            }
                            if let Some(req) = trace.request.clone(){ 
                                traceroute::process(req);
                            }
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::IcmpUnreachable(m) => {
                        trace!("ICMP-Unreachable: {}", m);

                        if let Some(trace) = tr_map.get_mut(&m.get_key()){
                            if let Some(hop) = trace.add_trace(&msg){
                                let msg = builder.create_hop_message(&[hop]);
                                println!("icmp-unreachable:{}\t{}->[{}]{}->\t{}",m.pkt_id, m.src,m.ttl,m.hop, m.dst);
                            }
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::Timer(now) =>{
                        //cleanup...
                        tr_map.values_mut()
                            .filter(|tr| tr.request.is_some())
                            .filter(|tr| {
                                if let Some(task) = tr.request.clone(){
                                    return now.duration_since(task.ts).as_secs() > 5
                                }
                                false
                            }).for_each(|tr| {
                                println!("timer: id:{} {} ttl:{}, hops:{}",tr.pkt_id, tr.dst, tr.ttl, tr.trace.len());
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