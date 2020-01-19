use std::sync::mpsc;

use pnet::datalink::{ self, Channel, linux};

use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::tcp::TcpOptionNumbers;

//use async_std::prelude::*;

#[macro_use] extern crate log;
extern crate lib_data;
pub use lib_data::*;

extern crate lib_tracer;

mod conf;


#[async_std::main]
async fn main() -> std::io::Result<()> {

    std::env::set_var("RUST_BACKTRACE", "1");

    //read command line...
    let mut opt = conf::OptConf::new();

    //setup logger...
    match opt.verbosity{
        0 => {},
        1 => std::env::set_var("RUST_LOG", "info"),
        2 => std::env::set_var("RUST_LOG", "debug"),
        _ => std::env::set_var("RUST_LOG", "trace"),

    }
    env_logger::init();

    //load from file...
    opt.load();

    if opt.iface.is_none(){
        error!("Network interface is not specified!");
        std::process::exit(-1);
    }

    let iface_name = opt.iface.unwrap();

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



    info!("Setting up interceptor on {} [{}]", net_iface.name, mac);
    info!("Detected networks:");
    net_iface.ips.iter()
        .for_each(|net| println!("\tnet:{}", net));


        
    let cfg = linux::Config::default();
    // cfg.fanout = Some(
    //     FanoutOption {
    //     group_id: 123,
    //     fanout_type: FanoutType::CPU,
    //     defrag: true,
    //     rollover: false,
    // });

    info!("About to create linux data channel...");
    let (_, mut rx) = match linux::channel(&net_iface, cfg) {
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



    //communication via channels
    let (data_sender, data_receiver): (lib_data::SenderChannel,lib_data::ReceiverChannel) = mpsc::channel();

    lib_tracer::start(data_receiver);

    info!("Starting listener loop...");
    loop{
        if let Ok(data) = rx.next(){ //this will timeout, as configured

            match EthernetPacket::new(data){
                Some(ethernet_packet) => {

                    match ethernet_packet.get_ethertype(){
                        EtherTypes::Ipv4 => {
                            if let Some(ip4pkt) = Ipv4Packet::new(ethernet_packet.payload()){

                                let next_proto = ip4pkt.get_next_level_protocol();
                                match next_proto{
                                    IpNextHeaderProtocols::Tcp => {
                                        if let Some(tcp) = TcpPacket::new(ip4pkt.payload()){
                                            let flags = tcp.get_flags();

                                            if tcp.get_options_iter().any(|o| o.get_number() == TcpOptionNumbers::SACK){
                                                let src = ip4pkt.get_source();
                                                let dst = ip4pkt.get_destination();
                                                warn!("re-transmission request detected: {} -> {}. Connection quality issues?", src, dst);
                                            }

                                            //SYN, SYN-ACK, ACK, u16
                                            if !has_bit(flags, Flags::SYN){//ignore all but SYN+ flag
                                                continue;
                                            }

                                            let src = ip4pkt.get_source(); //remote ip
                                            if net.contains(src){//originated from local network
                                                //SYN =>
                                                if !has_bit(flags, Flags::ACK){//SYN flag
                                                    let dst = ip4pkt.get_destination(); //local ip
                                                    data_sender.send(AppData::Syn(AppTarget{src,dst})).unwrap();
                                                }
    
                                                continue;
                                            }

                                            //SYN+ACK <=
                                            let dst = ip4pkt.get_destination(); //local ip
                                            data_sender.send(AppData::SynAck(AppTarget{src,dst})).unwrap();
                                            //println!("tcp {}->{} [{}]:{:?}",src, dst,decode(tcp.get_flags()), "tcp");
                                            continue;
                                        }

                                    }
                                    IpNextHeaderProtocols::Udp => {
                                        //println!("udp:{:?}", ip4pkt);
                                        continue
                                    }
                                    IpNextHeaderProtocols::Icmp => {
                                        if let Some(icmp) = IcmpPacket::new(ip4pkt.payload()){

                                            let t = icmp.get_icmp_type();

                                            match t{
                                                IcmpTypes::EchoReply => {
                                                    let src = ip4pkt.get_source();
                                                    let dst = ip4pkt.get_destination();
                                                    data_sender.send(AppData::IcmpReply(AppIcmp{src,dst})).unwrap();
                    
                                                }
                                                IcmpTypes::TimeExceeded => {
                                                    let src = ip4pkt.get_source();
                                                    let dst = ip4pkt.get_destination();
                                                    data_sender.send(AppData::IcmpExceeded(AppIcmp{src,dst})).unwrap();
                    
                                                }
                                                IcmpTypes::DestinationUnreachable => {
                                                    let src = ip4pkt.get_source();
                                                    let dst = ip4pkt.get_destination();
                                                    data_sender.send(AppData::IcmpUnreachable(AppIcmp{src,dst})).unwrap();
                    
                                                }
                                                _ => {
                                                    println!("icmp type:{:?}", t);
                                                    continue
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        //println!("next levep proto:{}", next_proto);
                                        continue
                                    }
                                }

                            }
                        }
                        EtherTypes::Ipv6 => {
                        }

                        _ => {
                        }
                    }

                }
                None => continue
            }
        }
    }
    //Ok(())
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