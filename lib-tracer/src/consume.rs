use std::sync::Arc;

use redis::Client;
use lib_data::{AppTraceRoute, AppHop};
use lib_fbuffers::Builder;

use async_std::task;


pub fn consume_route(client:Arc<Client>, _bldr:&mut Builder, route:& AppTraceRoute){
    
    let key = route.dst.clone().to_string();
    //let data = bldr.create_route_message(&[route.clone()]);
    let data = key.clone();

    task::spawn(async move {
        if let Err(e) = client.get_connection()
            .and_then(|mut conn| {

                redis::pipe()
                    .cmd("LPUSH")
                        .arg(key.clone())
                        .arg(data)
                        .ignore()
                    .cmd("EXPIRE")
                        .arg(key.clone())
                        .arg(1000 * 60)
                        .ignore()
                .query::<()>(&mut conn)

            }){
                error!("redis: unable to send! Error:{}", e);
            }
    
    });

    crate::traceroute::process(route.request.clone().unwrap());
}



pub fn consume_hop(client:Arc<Client>, _bldr:&mut Builder, hop:&AppHop){

    let key = hop.dst.clone().to_string();
    //let data = bldr.create_hop_message(&[hop.clone()]);
    let data = hop.hop.clone().to_string();

    task::spawn(async move {

        if let Err(e) = client.get_connection()
            .and_then(|mut conn| {
                redis::cmd("LPUSH")
                    .arg(key.clone()).arg(data)
                .query::<()>(&mut conn)
            }){
                error!("redis: unable to send! Error:{}", e);
            }
    
    });

}