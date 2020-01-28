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

        //let reader = maxminddb::Reader::open_readfile("/usr/local/share/GeoIP/GeoIP2-City.mmdb").unwrap();
        //let reader = maxminddb::Reader::open_readfile("/usr/share/GeoIP/GeoLite2-City.mmdb").unwrap();

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
                            let dup = tr_map.insert(trace.get_key(), trace);
                            if dup.is_some(){
                                warn!("syn dup:{}", m.dst);
                            }


                            //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
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
                            if let Some(task) = trace.add_trace(&msg){
                                trace.request = Some(task);
                                traceroute::process(trace.request.clone().unwrap());
                            }
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::IcmpExceeded(m) => {
                        trace!("ICMP-Exceeded: {}", m);

                        if let Some(trace) = tr_map.get_mut(&m.get_key()){
                            if let Some(task) = trace.add_trace(&msg){
                                trace.request = Some(task);
                                traceroute::process(trace.request.clone().unwrap());
                            }
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::IcmpUnreachable(m) => {
                        trace!("ICMP-Unreachable: {}", m);

                        if let Some(d) = tr_map.get_mut(&m.get_key()){
                            d.add_trace(&msg);
                        }else{
                            /* ignore aliens  */
                        }
                    }
                    AppData::Timer(_now) =>{

                        use lib_data::{TraceRouteNode, Node, Link};

                        let mut nodes = Vec::<Node>::new();
                        let mut links = Vec::<Link>::new();

                        tr_map.values_mut()
                            .filter(|trace| {
                                if trace.completed{

                                    let zz = trace.to();
                                    zz.nodes.into_iter().for_each(|n|{
                                        if !nodes.contains(&n){
                                            nodes.push(n);
                                        }
                                    });

                                    zz.links.into_iter().for_each(|n|{
                                        if !links.contains(&n){
                                            links.push(n);
                                        }
                                    });

                                    //warn!("trace completed:{}", trace.to_json());
                                }
                                !trace.completed
                            })
                            .for_each(|trace|{
                                if let Some(task) = trace.add_trace(&msg){
                                    trace.request = Some(task);
                                    traceroute::process(trace.request.clone().unwrap());
                                }
                            })
                        ;
                        println!("============================= start ================================================");
                        let zz = TraceRouteNode{nodes, links};
                        println!("{}", zz.to_json());
                        println!("============================= end ================================================");



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