#[macro_use] extern crate failure;
#[macro_use] extern crate log;

use std::sync::mpsc::{ Sender, Receiver };
use std::net::Ipv4Addr;
use std::fmt;
use std::collections::BTreeSet;
use std::time::Instant;

mod errors;
mod utils;
mod conf;

#[cfg(test)] mod tests;

pub use errors::*;
pub use utils::*;
pub use conf::*;

pub type ReceiverChannel = Receiver<AppData>;
pub type SenderChannel = Sender<AppData>;


const MAX_TTL:u8 = 64;

#[derive(Debug, Clone)]
pub enum AppData{
    Syn(AppTcp),
    SynAck(AppTcp),

    IcmpReply(AppIcmp),
    IcmpExceeded(AppIcmp),
    IcmpUnreachable(AppIcmp),
    Timer(Instant),

}


#[derive(Debug, Clone)]
pub struct AppTcp{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub outbound: bool,
    
    pub id:u16,

    pub syn_ts: Option<Instant>,
    pub synack_ts: Option<Instant>,
}

impl AppTcp{
    pub fn new(src:Ipv4Addr, dst:Ipv4Addr, outbound:bool, syn_ts:Option<Instant>, synack_ts:Option<Instant>) -> Self{
        AppTcp{src,dst,outbound, id:0, syn_ts, synack_ts}
    }

    //always remote...
    pub fn get_key(&self) -> &Ipv4Addr { 
        if self.outbound{
            &self.dst
        }else{
            &self.src
        }
    }

    pub fn apply(&mut self, v:&AppData){
        match v{
            AppData::Syn(v) => {

                self.syn_ts = v.syn_ts.or(self.syn_ts);
                debug!("ACK    : {}", self);
            }
            AppData::SynAck(v) => {
                self.synack_ts = v.synack_ts.or(self.synack_ts);
                debug!("SYN-ACK: {}", self);
            }
            _ => ()
        }

    }
}

impl fmt::Display for AppTcp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed = self.synack_ts.map_or_else(||0,|d|d.elapsed().as_millis());
        write!(f, "key:{}, id:{}  {} -> {}, elapsed:{:?}", self.get_key(), self.id, self.src, self.dst, elapsed)
    }
}

#[derive(Debug, Clone)]
pub struct AppIcmp{
    pub src: Ipv4Addr, //this ip
    pub dst: Ipv4Addr, //intended target
    pub hop: Ipv4Addr, //

    pub pkt_id: u16,
    pub pkt_seq: u16,

    pub ttl:u8,
    pub ts: Instant,
}

impl AppIcmp{
    //this_ip+pkt_id
    pub fn get_key(&self) -> Ipv4Addr { self.dst }
}

impl fmt::Display for AppIcmp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {} -> {} [id:{},seq/ttl:{}, reverse ttl:{}]", self.src, self.hop, self.dst, self.pkt_id, self.pkt_seq, self.ttl)
    }
}

#[derive(Debug, Clone)]
pub struct AppTraceRoute{
    // this_ip - mid - dst
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub trace: BTreeSet<AppHop>,
    pub pkt_id: u16,

    pub ttl:u8,
    //route: local_ip + id -> dst
    // replies come to this_ip + id. EchoRequests to dst. from this_ip with identifier=id

    pub request:Option<AppTraceRouteTask>,
}

impl AppTraceRoute{
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, pkt_id:u16) -> Self{
        let mut ret_val = AppTraceRoute{src, dst, pkt_id, trace:BTreeSet::<AppHop>::new(), ttl:1u8, request:None};
        ret_val.request = Some(AppTraceRouteTask::from(&ret_val));
        ret_val
    }

    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    fn next_ttl(&mut self) -> bool{
        
        if self.request.is_some(){
            self.ttl += 1;
            if self.ttl > MAX_TTL {
                self.request = None;
            }
        }
        self.request.is_some()
    }

    fn get_rtt(&self, m:&AppIcmp) -> u16{
        if let Some(req) = self.request.clone(){
            if req.pkt_id == m.pkt_id{
            m.ts.duration_since(req.ts).as_millis() as u16    
            }else{
                std::u16::MAX
            }
        }else{
            std::u16::MAX
        }
    }

    pub fn setup_for_next_request(&mut self){
        if self.next_ttl(){
            self.request = Some(AppTraceRouteTask::from(&*self));
        }else{
            self.request = None;
        }
    }

    pub fn add_trace(&mut self, data:&AppData) -> Option<AppHop>{
        
        match data{
            AppData::IcmpReply(m)  =>{
                debug!("ICMP-Reply: {}", m);
                
                let hop = AppHop::new(m.pkt_seq as u8, m.pkt_id, m.hop, self.get_rtt(m));
                
                if !self.trace.contains(&hop){
                    self.trace.insert(hop.clone());
                    if m.hop == self.dst{
                        self.request = None;
                    }else{
                        self.setup_for_next_request();
                    }
                    return Some(hop);
                }

            }
            AppData::IcmpExceeded(m)  => {
                debug!("ICMP-Exceeded: {}", m);

                let hop = AppHop::new(m.pkt_seq as u8, m.pkt_id, m.hop, self.get_rtt(m)); //pkt.ttl is reverse ttl and is not reliable...
                if !self.trace.contains(&hop){
                    self.trace.insert(hop.clone()); //self.trace.len() + 1 = next ttl
                    if m.hop == self.dst{
                        self.request = None;
                    }else{
                        self.setup_for_next_request();
                    }

                    return Some(hop);
                }
            }
            AppData::IcmpUnreachable(m)  =>{
                debug!("ICMP-Unreachable: {}", m);

                let hop = AppHop::new(m.pkt_seq as u8, m.pkt_id, m.hop, self.get_rtt(m));
                if !self.trace.contains(&hop){
                    self.trace.insert(hop.clone());
                    if m.hop == self.dst{
                        self.request = None;
                    }else{
                        self.setup_for_next_request();
                    }
                    return Some(hop);
                }
            }
            //AppData::Timer(_now)  =>()
            _ => ()
        }
        None
    }
}
impl fmt::Display for AppTraceRoute{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let trace = self.trace.iter()
            .map(|e|format!("{},", e))
            .fold(String::new(), |mut a, e| {a.push_str(&e); a})
            ;
        write!(f, "route[{}]: {} -> [{}] -> {} [id:{}, next seq/ttl:{}, discovered hops:{}]", self.pkt_id, self.src, trace, self.dst, self.pkt_id, self.ttl, self.trace.len())
    }
}

#[derive(Debug, Clone)]
pub struct AppTraceRouteTask{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,

    pub pkt_id: u16,
    pub pkt_seq: u16,

    pub ttl:u8,
    pub ts: Instant,

}

impl From<& AppTraceRoute> for AppTraceRouteTask {
    fn from(from: & AppTraceRoute) -> Self {
        
        AppTraceRouteTask{
            src:from.src,
            dst:from.dst,
            pkt_id: from.pkt_id,
            pkt_seq: from.ttl as u16, //from.trace.len() as u16,
            ttl: from.ttl,
            ts:Instant::now()
        }
    }
}

impl fmt::Display for AppTraceRouteTask{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {} [id:{}, seq:{}, ttl:{}] elapsed:{}", self.src, self.dst, self.pkt_id, self.pkt_seq, self.ttl, self.ts.elapsed().as_millis())
    }
}




#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppHop{
    pub ttl:u8,
    pub pkt_id: u16,
    pub hop: Ipv4Addr,
    pub rtt: u16,
}
impl AppHop{
    pub fn new(ttl:u8, pkt_id: u16, hop:Ipv4Addr, rtt:u16) -> Self{
        AppHop{ttl,pkt_id, hop, rtt}
    }
}
impl fmt::Display for AppHop{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ttl, self.hop)
    }
}