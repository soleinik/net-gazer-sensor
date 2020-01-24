#[macro_use] extern crate failure;
#[macro_use] extern crate log;

use std::sync::mpsc::{ Sender, Receiver };
use std::net::Ipv4Addr;
use std::fmt;
use std::collections::BTreeSet;
mod errors;

pub use errors::*;


pub type ReceiverChannel = Receiver<AppData>;
pub type SenderChannel = Sender<AppData>;

#[derive(Debug, Clone)]
pub enum AppData{
    Syn(AppTcp),
    SynAck(AppTcp),

    IcmpReply(AppIcmp),
    IcmpExceeded(AppIcmp),
    IcmpUnreachable(AppIcmp),
}


#[derive(Debug, Clone)]
pub struct AppTcp{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub outbound: bool,
    
    pub id:u16
}

impl AppTcp{
    pub fn new(src:Ipv4Addr, dst:Ipv4Addr, outbound:bool) -> Self{
        AppTcp{src,dst,outbound, id:0}
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
                debug!("ACK    : {}", self);
            }
            AppData::SynAck(v) => {
                debug!("SYN-ACK: {}", self);
            }
            _ => ()
        }

    }
}

impl fmt::Display for AppTcp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key:{}, id:{}  {} -> {}", self.get_key(), self.id, self.src, self.dst)
    }
}

#[derive(Debug, Clone)]
pub struct AppIcmp{
    pub src: Ipv4Addr, //this ip
    pub dst: Ipv4Addr, //intended target
    pub hop: Ipv4Addr, //

    pub pkt_id: u16,
    pub pkt_seq: u16,

    pub ttl:u8
}

impl AppIcmp{
    //this_ip+pkt_id
    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    pub fn apply(&mut self, v:&AppData){

    }
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
    pub completed:bool
    //route: local_ip + id -> dst
    // replies come to this_ip + id. EchoRequests to dst. from this_ip with identifier=id
}

impl AppTraceRoute{
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, pkt_id:u16) -> Self{
        AppTraceRoute{src, dst, pkt_id, trace:BTreeSet::<AppHop>::new(), ttl:1u8, completed:false}
    }
    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    pub fn add_trace(&mut self, data:&AppData) -> Option<AppTraceRouteTask>{

        match data{
            AppData::IcmpReply(m)  =>{
                debug!("ICMP-Reply: {}", m);
                let hop = AppHop::new(m.ttl, m.hop);
                if !self.trace.contains(&hop){
                    self.trace.insert(hop);
                    info!("start {}", self);
                    return None;
                }
            }
            AppData::IcmpExceeded(m)  => {
                debug!("ICMP-Exceeded: {}", m);

                let hop = AppHop::new(m.pkt_seq as u8, m.hop); //pkt.ttl is reverse ttl and is not reliable...
                if !self.trace.contains(&hop){
                    self.trace.insert(hop); //self.trace.len() + 1 = next ttl
                    self.ttl += 1;
                    info!("{}", self);
                    return Some(AppTraceRouteTask::from(&*self));
                }
            }
            AppData::IcmpUnreachable(m)  =>{
                debug!("ICMP-Unreachable: {}", m);
                let hop = AppHop::new(m.ttl, m.hop);
                if !self.trace.contains(&hop){
                    self.trace.insert(hop);
                    self.completed = true; //maybe compare hope to dst...
                    info!("done {}", self);
                }
            }
            _ => ()
        }
        None
    }
}
impl fmt::Display for AppTraceRoute{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let trace = self.trace.iter()
            .map(|e|format!("{}, ", e))
            .fold(String::new(), |mut a, e| {a.push_str(&e); a})
            ;
        write!(f, "route: {} -> [{}] -> {} [id:{}, next seq/ttl:{}]", self.src, trace, self.dst, self.pkt_id, self.ttl)
    }
}


#[derive(Debug, Clone)]
pub struct AppTraceRouteTask{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,

    pub pkt_id: u16,
    pub pkt_seq: u16,

    pub ttl:u8
}

impl From<&AppTraceRoute> for AppTraceRouteTask {
    fn from(from: &AppTraceRoute) -> Self {
        
        AppTraceRouteTask{
            src:from.src,
            dst:from.dst,
            pkt_id: from.pkt_id,
            pkt_seq: from.ttl as u16, //from.trace.len() as u16,
            ttl: from.ttl
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct AppHop{
    pub ttl:u8,
    pub hop: Ipv4Addr,

}
impl AppHop{
    pub fn new(ttl:u8, hop:Ipv4Addr) -> Self{
        AppHop{ttl, hop}
    }
}
impl fmt::Display for AppHop{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ttl, self.hop)
    }
}


