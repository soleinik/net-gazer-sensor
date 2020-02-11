use lib_data::{AppTraceRoute, AppHop};
use lib_fbuffers::Builder;

use async_std::task;

pub fn consume_route(bldr:&mut Builder, route:& AppTraceRoute){
    let data = bldr.create_route_message(&[route.clone()]);

    let rte = route.clone();
    task::spawn(async move {
        let resp = ureq::post("http://127.0.0.1:8080/data")
        //.set("X-My-Header", "Secret")
        .send_bytes(&data);

        trace!("route[{}] http response:{:?}",rte, resp);
    });

    crate::traceroute::process(route.request.clone().unwrap());
}



pub fn consume_hop(bldr:&mut Builder, hop:&AppHop){
    let data = bldr.create_hop_message(&[hop.clone()]);
    let h = hop.clone();
    task::spawn(async move {
        let resp = ureq::post("http://127.0.0.1:8080/data")
        //.set("X-My-Header", "Secret")
        .send_bytes(&data);
        trace!("hop[{}] http response:{:?}",h, resp);
    
    });
}
