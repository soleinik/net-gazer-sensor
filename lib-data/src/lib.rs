#[macro_use] extern crate failure;

use std::sync::mpsc::{ Sender, Receiver };
use std::net::Ipv4Addr;
use std::fmt;
mod errors;

pub use errors::*;


pub type ReceiverChannel = Receiver<AppData>;
pub type SenderChannel = Sender<AppData>;

pub trait Data<K, V>{
    fn get_key(&self) -> &Ipv4Addr;
    fn apply(&mut self, v:&V);
} 


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
    //local
    pub dst: Ipv4Addr,

    pub outbound: bool
}

impl Data<Ipv4Addr, AppData> for AppTcp{
    fn get_key(&self) -> &Ipv4Addr { 
        if self.outbound{
            &self.dst
        }else{
            &self.src

        }
    }

    fn apply(&mut self, v:&AppData){

    }
}

impl fmt::Display for AppTcp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "key:{}  {} -> {}", self.get_key(), self.src, self.dst)
    }
}


#[derive(Debug, Clone)]
pub struct AppIcmp{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,

    pub outbound: bool
}

impl Data<Ipv4Addr, AppData> for AppIcmp{
    fn get_key(&self) -> &Ipv4Addr { 
        if self.outbound{
            &self.dst
        }else{
            &self.src

        }
    }

    fn apply(&mut self, v:&AppData){

    }
}

impl fmt::Display for AppIcmp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.outbound{
            write!(f, "{} -> {}", self.src, self.dst)
        }else {
            write!(f, "{} -> {}", self.dst, self.src)
        }
    }
}