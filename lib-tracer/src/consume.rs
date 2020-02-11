use lib_data::{AppTraceRoute, AppHop};
use lib_fbuffers::Builder;

use async_std::task;

pub fn consume_route(bldr:&mut Builder, route:& AppTraceRoute, url:&str){
    let data = bldr.create_route_message(&[route.clone()]);

    let rte = route.clone();
    let u = url.to_owned();

    task::spawn(async move {
        let resp = ureq::post(&u)
        //.set("X-My-Header", "Secret")
        .send_bytes(&data);

        println!("route[{}] http response:{:?} url:{}",rte, resp, u);
    });

    crate::traceroute::process(route.request.clone().unwrap());
}



pub fn consume_hop(bldr:&mut Builder, hop:&AppHop, url:&str){
    let data = bldr.create_hop_message(&[hop.clone()]);
    let h = hop.clone();
    let u = url.to_owned();

    task::spawn(async move {
        let resp = ureq::post(&u)
        //.set("X-My-Header", "Secret")
        .send_bytes(&data);
        println!("hop[{}] http response:{:?} url:{}",h, resp, u);
    
    });
}
