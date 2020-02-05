#![allow(dead_code)]

use flatbuffers::FlatBufferBuilder;

mod traceroute_generated;
use traceroute_generated::*;


pub struct Builder<'fbb>{
    bldr: FlatBufferBuilder<'fbb>,
}

impl<'a> Default for Builder<'a> {
    #[inline]
    fn default() -> Self {
        Builder { bldr: FlatBufferBuilder::new() }
    }
}

impl Builder<'_> {

    pub fn make_route(&mut self, args:& RouteArgs) -> Vec<u8>{

        let mut msg = Vec::<u8>::new();
        self.bldr.reset();
    
        // let mut args = RouteArgs::default();
        // args.route_id = 0;
        // args.src = None;
        // args.dst = None;
        // args.max_ttl = 0;
        // args.hops = None;
    
    
        let route_offset = Route::create(&mut self.bldr, &args);
    
        finish_route_buffer(&mut self.bldr, route_offset);
        let finished_data = self.bldr.finished_data();
        msg.extend_from_slice(finished_data);
        msg
    }
}



