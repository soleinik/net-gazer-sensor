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
    Syn(AppTarget),
    SynAck(AppTarget),
    IcmpReply(AppIcmp),
    IcmpExceeded(AppIcmp),
    IcmpUnreachable(AppIcmp),
}


#[derive(Debug, Clone)]
pub struct AppTarget{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr
}

impl fmt::Display for AppTarget{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.src, self.dst)
    }
}


#[derive(Debug, Clone)]
pub struct AppIcmp{
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr
}

impl fmt::Display for AppIcmp{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.src, self.dst)
    }
}