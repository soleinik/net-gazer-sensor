#[macro_use] extern crate log;
//#[macro_use] 
extern crate packet_builder;

//use maxminddb::geoip2;

mod traceroute;

use std::thread;
use lib_data::{AppData, ReceiverChannel};


pub fn start(rx:ReceiverChannel){
    info!("Starting tracer loop...");
    thread::spawn(move || {

        //let reader = maxminddb::Reader::open_readfile("/usr/local/share/GeoIP/GeoIP2-City.mmdb").unwrap();
        //let reader = maxminddb::Reader::open_readfile("/usr/share/GeoIP/GeoLite2-City.mmdb").unwrap();

        loop{
            if let Ok(msg) = rx.recv(){
                match msg {
                    AppData::Syn(msg) => {
                        //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
                        info!("SYN: {}", msg); //,to_string(city));
                    }
                    AppData::SynAck(msg) => {
                        //let city: Option<geoip2::City> = reader.lookup(std::net::IpAddr::V4(msg.dst)).ok();
                        info!("SYN-ACK: {}", msg); //,to_string(city));
                    }
                    AppData::IcmpReply(msg) => {
                        info!("ICMP-Reply: {}", msg)
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