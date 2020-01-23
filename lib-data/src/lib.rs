#[macro_use] extern crate failure;

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

    pub fn get_key(&self) -> &Ipv4Addr { 
        if self.outbound{
            &self.dst
        }else{
            &self.src
        }
    }

    pub fn apply(&mut self, v:&AppTcp){
        println!("apply{} to:{:?}", self, v);

    }
}

impl fmt::Display for AppTcp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key:{}, id:{}  {} -> {}", self.get_key(),self.id, self.src, self.dst)
    }
}

#[derive(Debug, Clone)]
pub struct AppIcmp{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,

    pub pkt_id: u16,
    pub pkt_seq: u16

}

impl AppIcmp{
    pub fn get_key(&self) -> (&Ipv4Addr,u16) { (&self.dst, self.pkt_id) }

    pub fn apply(&mut self, v:&AppIcmp){

    }
}

impl fmt::Display for AppIcmp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {} [id:{},seq:{}]", self.src, self.dst, self.pkt_id, self.pkt_seq)
    }
}