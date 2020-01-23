#[macro_use] extern crate log;
//extern crate crossbeam_deque;
//use maxminddb::geoip2;

mod traceroute;

use std::thread;
use std::collections::HashMap;
use std::net::Ipv4Addr;

use lib_data::{AppData, ReceiverChannel, AppTcp, AppIcmp};


pub fn start(rx:ReceiverChannel){
    info!("Starting tracer loop...");
    thread::spawn(move || {

        // some local net ip  and remote ip
        let mut tcp_map = HashMap::<Ipv4Addr, AppTcp>::new();


        //local net ip+id
        let mut tr_map = HashMap::<(Ipv4Addr, u16), AppIcmp>::new();


        //let reader = maxminddb::Reader::open_readfile("/usr/local/share/GeoIP/GeoIP2-City.mmdb").unwrap();
        //let reader = maxminddb::Reader::open_readfile("/usr/share/GeoIP/GeoLite2-City.mmdb").unwrap();

        let mut  id_seq = 0u16; //0-65535

        loop{
            if let Ok(msg) = rx.recv(){
                match msg.clone() {
                    AppData::Syn(mut m) => { //outbound, use dst
                        if let Some(d) = tcp_map.get_mut(m.get_key()){
                            d.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);
                            tcp_map.insert(*m.get_key(), m.clone());
                            //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
                            info!("SYN    : {}", m); //,to_string(city));
                        }
                    }
                    AppData::SynAck(mut m) => { //inbound, use src
                        if let Some(d) = tcp_map.get_mut(m.get_key()){
                            d.apply(&msg);
                        }else{
                            m.id = id_seq;
                            id_seq = increment(id_seq);
                            tcp_map.insert(*m.get_key(), m.clone());
                            //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
                            info!("SYN-ACK: {}", m); //,to_string(city));
                        }
                    }
                    AppData::IcmpReply(m) => {

                        info!("ICMP-Reply: {}", m)
                    }
                    AppData::IcmpExceeded(msg) => {

                        info!("ICMP-Exceeded: {}", msg)
                    }
                    AppData::IcmpUnreachable(msg) => {

                        info!("ICMP-unreachable: {}", msg)
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