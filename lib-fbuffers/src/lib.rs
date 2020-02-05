#![allow(dead_code)]

use flatbuffers::FlatBufferBuilder;

mod traceroute_generated;
use traceroute_generated::*;


use lib_data::*;

pub struct Builder<'fbb>{
    seq: u64,
    bldr: FlatBufferBuilder<'fbb>,
}

impl<'a> Default for Builder<'a> {
    #[inline]
    fn default() -> Self {
        Builder { seq:0, bldr: FlatBufferBuilder::new()}
    }
}

impl Builder<'_> {

    fn reset(&mut self){
        self.bldr.reset();
        self.seq += 1; //FIXME: overflow
    }

    pub fn create_hop_message(&mut self, hops:&[AppHop]) -> Vec<u8>{
        let mut msg = Vec::<u8>::new();
        self.bldr.reset();

        let mut args = MessageArgs::default();
        args.seq = self.seq;

        let hops_vec:Vec<flatbuffers::WIPOffset<Hop>> = 
            hops.iter()
                .map(|x| {
                    Hop::create(&mut self.bldr, &HopArgs::from(x))
                })
                .collect();

        args.hops = Some(self.bldr.create_vector(&hops_vec));

        let message_offset = Message::create(&mut self.bldr, &args);
        finish_message_buffer(&mut self.bldr, message_offset);
        let finished_data = self.bldr.finished_data();
        msg.extend_from_slice(finished_data);
        msg
    }

    pub fn create_route_message(&mut self, routes:&[AppTraceRoute]) -> Vec<u8>{

        let mut msg = Vec::<u8>::new();
        self.bldr.reset();
    
        let mut args = MessageArgs::default();
        args.seq = self.seq;

        let rts_vec:Vec<flatbuffers::WIPOffset<Route>> = 
            routes.iter()
                .map(|x| {
                    Route::create(&mut self.bldr, &RouteArgs::from(x))
                })
                .collect();

        args.routes = Some(self.bldr.create_vector(&rts_vec));

        let message_offset = Message::create(&mut self.bldr, &args);
        finish_message_buffer(&mut self.bldr, message_offset);
        let finished_data = self.bldr.finished_data();
        msg.extend_from_slice(finished_data);
        msg
    }
}

impl From<& AppTraceRoute> for RouteArgs {
    fn from(from: & AppTraceRoute) -> Self {
        RouteArgs{
            route_id: from.pkt_id,
            src: from.src.into(),
            dst: from.dst.into()
        }
    }
}

impl From<& AppHop> for HopArgs {
    fn from(from: & AppHop) -> Self {
        HopArgs{
            hop: from.hop.into(),
            ttl: from.ttl,
            route_id: from.pkt_id,
            rtt: from.rtt
        }
    }
}
