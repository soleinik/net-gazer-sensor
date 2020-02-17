use std::sync::mpsc;

use pnet::datalink::{ self, Channel, Config, channel};

use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{TcpPacket, TcpOptionNumbers};
use pnet::packet::icmp::{
    IcmpPacket, IcmpTypes, 
    echo_reply::EchoReplyPacket, 
    time_exceeded::TimeExceededPacket, 
    echo_request::EchoRequestPacket, 
    destination_unreachable::DestinationUnreachablePacket
};

use std::time::Instant;

//use async_std::prelude::*;

#[macro_use] extern crate log;
extern crate lib_data;
pub use lib_data::*;

extern crate lib_tracer;
extern crate lib_fbuffers;
extern crate lib_comm;
extern crate lib_plugins;


#[async_std::main]
async fn main() -> std::io::Result<()> {

    std::env::set_var("RUST_BACKTRACE", "1");

    //read command line...
    let mut opt = lib_data::OptConf::default();

    //setup logger...
    match opt.verbosity{
        0 => std::env::set_var("RUST_LOG", "warn"),
        1 => std::env::set_var("RUST_LOG", "info"),
        2 => std::env::set_var("RUST_LOG", "debug"),
        _ => std::env::set_var("RUST_LOG", "trace"),

    }
    env_logger::init();

    //load from file...
    opt.load(env!("CARGO_PKG_NAME"));
    opt.validate().unwrap();

    let iface_name = opt.iface.clone().unwrap();

    let net_iface = 
        datalink::interfaces().into_iter()
            .filter(|iface| iface.is_up())
            .filter(|iface| !iface.ips.is_empty())
            .find(| iface | iface.name == iface_name)
            .unwrap_or_else(|| {
                error!("Invalid Network Interface. No active device '{}'",iface_name);
                std::process::exit(-1);
            });

    let mac = net_iface.mac_address();

    //need network
    let net = net_iface.ips.iter()
        .map(|net| {
            match net{
                ipnetwork::IpNetwork::V4(net)=> Some(net),
                _ => None
            }
        })
        .find(|net| net.is_some()).flatten().unwrap();

    //need ip
    let ip = net.ip();
    

    info!("Setting up interceptor on {} [{}]", net_iface.name, mac);
    info!("Detected networks:");
    net_iface.ips.iter()
        .for_each(|net| println!("\tnet:{}", net));

    info!("net:{}", net);
        
    let cfg = Config::default();
    // cfg.fanout = Some(
    //     FanoutOption {
    //     group_id: 123,
    //     fanout_type: FanoutType::CPU,
    //     defrag: true,
    //     rollover: false,
    // });

    let plugins = lib_plugins::PluginManager::new();
    if plugins.is_empty(){
        error!("No plugins found! System is not operational - aborting...");
        std::process::exit(-4);
    }


    info!("About to create ethernet link channel...");
    let (_, mut rx) = match channel(&net_iface, cfg) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            error!("Unable to open data link channel! Unexpected data link channel type");
            std::process::exit(-2);
        }
        Err(e) => {
            error!("Unable to open data link channel! Error:{}", e);
            std::process::exit(-3);
        }
    };

    //reporting...
    let (comm_sender, comm_receiver): (lib_comm::CommTxChannel,lib_comm::CommRxChannel) = mpsc::channel();
    lib_comm::start(comm_receiver, &opt);

    // //communication via async channels - unbounded queue, watch for OOM. 
    // // 1:1 producer:consumer
    // let (data_sender, data_receiver): (lib_data::SenderChannel,lib_data::ReceiverChannel) = mpsc::channel();

    // lib_tracer::start(data_receiver, ip, comm_sender.clone());
    // lib_tracer::timer_start(data_sender);


    info!("Starting listener loop...");
    loop{
        if let Ok(data) = rx.next(){ //this will timeout, as configured
            match EthernetPacket::new(data){
                Some(ethernet_packet) => {
                    plugins.process(comm_sender.clone(), &ethernet_packet);
                }
                None => continue
            }
        }
    }
}

use pnet::packet::tcp::TcpFlags;
#[macro_use] extern crate bitflags;

bitflags! {
    struct Flags: u16 {
        const SYN = TcpFlags::SYN; //2
        const URG = TcpFlags::URG; //32
        const ACK = TcpFlags::ACK; //16
        const PSH = TcpFlags::PSH; //8
        const RST = TcpFlags::RST; //4
        const FIN = TcpFlags::FIN; //1

        const CWR = TcpFlags::CWR; //
        const ECE = TcpFlags::ECE; //
    }
}

fn has_bit(flags:u16, bit:Flags) -> bool{
    if let Some(s) = Flags::from_bits(flags){
        return s.contains(bit);
    }
    false
}

// fn decode(flags:u16)-> String{
//     format!("{:?}", Flags::from_bits(flags).unwrap())

// }