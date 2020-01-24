#[macro_use] extern crate failure;
#[macro_use] extern crate log;

use std::sync::mpsc::{ Sender, Receiver };
use std::net::Ipv4Addr;
use std::fmt;
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
        write!(f, "{} -> {} -> {} [id:{},seq:{}, reverse ttl:{}]", self.src, self.hop, self.dst, self.pkt_id, self.pkt_seq, self.ttl)
    }
}

#[derive(Debug, Clone)]
pub struct AppTraceRoute{
    // this_ip - mid - dst
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub trace: Vec<Ipv4Addr>,
    pub pkt_id: u16,

    pub ttl:u8
    //route: local_ip + id -> dst
    // replies come to this_ip + id. EchoRequests to dst. from this_ip with identifier=id
}

impl AppTraceRoute{
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, pkt_id:u16) -> Self{
        //ttl=64 will be ICMP Ping
        //AppTraceRoute{src, dst, pkt_id, trace:Vec::new(), ttl:std::u8::MAX}
        AppTraceRoute{src, dst, pkt_id, trace:Vec::new(), ttl:1u8}
    }
    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    pub fn add_trace(&mut self, data:&AppData) -> Option<AppTraceRouteTask>{

        match data{
            AppData::IcmpExceeded(m)  => {
                debug!("ICMP-Exceeded: {}", m);
                if !self.trace.contains(&m.hop){
                    self.trace.push(m.hop); //self.trace.len() + 1 = next ttl
                    self.ttl += 1;
                    info!("{}", self);
                    return Some(AppTraceRouteTask::from(&*self));
                }
            }
            AppData::IcmpUnreachable(m)  =>{
                debug!("ICMP-Unreachable: {}", m);
                if !self.trace.contains(&m.hop){
                    self.trace.push(m.hop);
                    info!("done {}", self);
                }
            }
            AppData::IcmpReply(m)  =>{
                debug!("ICMP-Reply: {}", m);
                if !self.trace.contains(&m.hop){
                    self.trace.push(m.hop);
                    info!("start {}", self);
                    return None;
                }
            }
            _ => ()
        }
        None
    }
}
impl fmt::Display for AppTraceRoute{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "route:{} -> {:?} -> {} [id:{},seq:{}]", self.src, self.trace, self.dst, self.pkt_id, self.ttl)
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
            pkt_seq: from.trace.len() as u16,
            ttl: from.ttl
        }
    }
}