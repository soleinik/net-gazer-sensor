#[macro_use] extern crate failure;
#[macro_use] extern crate log;

use std::sync::mpsc::{ Sender, Receiver };
use std::net::Ipv4Addr;
use std::fmt;
use std::collections::BTreeSet;
use std::time::Instant;

mod errors;

#[cfg(test)] mod tests;

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
        let elapsed = self.synack_ts.and_then(|sa|self.syn_ts.map(|s| sa.duration_since(s)) );
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

    pub ttl:u8
}

impl AppIcmp{
    //this_ip+pkt_id
    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    pub fn apply(&mut self, _v:&AppData){

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

    ttl:u8,
    pub completed:bool,
    //route: local_ip + id -> dst
    // replies come to this_ip + id. EchoRequests to dst. from this_ip with identifier=id

    pub request:Option<AppTraceRouteTask>,
}

impl AppTraceRoute{
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, pkt_id:u16) -> Self{
        AppTraceRoute{src, dst, pkt_id, trace:BTreeSet::<AppHop>::new(), ttl:1u8, completed:false, request:None}
    }
    pub fn get_key(&self) -> Ipv4Addr { self.dst }

    fn next_ttl(&mut self) -> bool{
        if !self.completed{
            self.ttl += 1;
            if self.ttl > 254 {
                self.completed = true;
            }
        }
        !self.completed
    }

    pub fn add_trace(&mut self, data:&AppData) -> Option<AppTraceRouteTask>{

        match data{
            AppData::IcmpReply(m)  =>{
                debug!("ICMP-Reply: {}", m);
                let hop = AppHop::new(m.ttl, m.hop);
                if !self.trace.contains(&hop){
                    if m.hop == self.dst{
                        self.completed = true;
                    }
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
                    if m.hop == self.dst{
                        self.completed = true;
                        return None;
                    }
                    if self.next_ttl() {
                        info!("{}", self);
                        return Some(AppTraceRouteTask::from(&*self));
                    }
                }
            }
            AppData::IcmpUnreachable(m)  =>{
                debug!("ICMP-Unreachable: {}", m);
                let hop = AppHop::new(m.ttl, m.hop);
                if !self.trace.contains(&hop){
                    self.trace.insert(hop);
                    if m.hop == self.dst{
                        self.completed = true;
                    }
                    self.completed = true; //maybe compare hope to dst...
                    warn!("done {}", self);
                }
            }
            AppData::Timer(now)  =>{
                if !self.completed {
                    if let Some(task) = self.request.clone(){
                        if now.duration_since(task.ts).as_secs() > 5 && self.next_ttl(){
                            info!("timer: id:{} {} ttl:{}, hops:{}",self.pkt_id, self.dst, self.ttl, self.trace.len());
                            return Some(AppTraceRouteTask::from(&*self));
                        }
                    }
                }
                return None;
            }
            _ => ()
        }
        None
    }
    pub fn to_json(&self) -> String{
        let nodes = TraceRouteNode::from(self);
        serde_json::to_string(&nodes).unwrap()
    }

    pub fn to(&self) -> TraceRouteNode{
        TraceRouteNode::from(self)
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




/*
  {
      "id": "Myriel",
      "group": 1
    },

 {
      "source": "Napoleon",
      "target": "Myriel",
      "value": 1
    },


*/
use serde::{Serialize};

#[derive(Serialize, Debug)]
pub struct Node{
    pub id:String,
    pub group:u16
}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Debug)]
pub struct Link{
    pub source:String,
    pub target:String,
    pub value:u16 //group
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.target == other.target
    }
}


#[derive(Serialize, Debug)]
pub struct TraceRouteNode{
    pub nodes:Vec<Node>,
    pub links:Vec<Link>,

}

impl TraceRouteNode{
    pub fn to_json(&self) -> String{
        serde_json::to_string(self).unwrap()
    }

}

impl From<& AppTraceRoute> for TraceRouteNode {
    fn from(from: & AppTraceRoute) -> Self {
        let mut nodes = Vec::<Node>::new();

        nodes.push(Node{
            id:format!("{}", from.src),
            group:from.pkt_id
        });

        from.trace.iter().for_each(|hop|{
            nodes.push(Node{
                id:format!("{}", hop.hop),
                group:from.pkt_id
            });
        });

        nodes.push(Node{
            id:format!("{}", from.dst),
            group:from.pkt_id
        });


        let mut links = Vec::<Link>::new();

        let mut src = format!("{}", from.src);
        from.trace.iter().for_each(|hop|{
            let dst = format!("{}", hop.hop);
            let link = Link{
                source:src.clone(),
                target:dst.clone(),
                value:from.pkt_id
            };
            links.push(link);
            src = dst;
        });




        TraceRouteNode{nodes, links}
    }
}
