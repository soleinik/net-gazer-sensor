use std::sync::mpsc;

use pnet::datalink::{ self, Channel, linux};

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

    //need ip
    let ip = net.ip();
    

    info!("Setting up interceptor on {} [{}]", net_iface.name, mac);
    info!("Detected networks:");
    net_iface.ips.iter()
        .for_each(|net| println!("\tnet:{}", net));

    info!("net:{}", net);
        
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



    //communication via async channels - unbounded queue, watch for OOM. 
    // 1:1 producer:consumer
    let (data_sender, data_receiver): (lib_data::SenderChannel,lib_data::ReceiverChannel) = mpsc::channel();


    lib_tracer::start(data_receiver, ip);

    info!("Starting listener loop...");
    loop{
        if let Ok(data) = rx.next(){ //this will timeout, as configured

            match EthernetPacket::new(data){
                Some(ethernet_packet) => {

                    match ethernet_packet.get_ethertype(){
                        EtherTypes::Ipv4 => {
                            if let Some(ip4pkt) = Ipv4Packet::new(ethernet_packet.payload()){

                                match ip4pkt.get_next_level_protocol(){

                                    IpNextHeaderProtocols::Tcp => {
                                        if let Some(tcp) = TcpPacket::new(ip4pkt.payload()){
                                            let flags = tcp.get_flags();

                                            if 0 == tcp.get_window(){
                                                let src_port = tcp.get_source();
                                                let dst_port = tcp.get_destination();
                                                let src = ip4pkt.get_source();
                                                let dst = ip4pkt.get_destination();
                                                trace!("source overloaded {}:{} -> {}:{}. Application performance issues?", src,src_port, dst, dst_port);
                                            }

                                            if tcp.get_options_iter().any(|o| o.get_number() == TcpOptionNumbers::SACK){

                                                //let mss = tcp.get_options_iter().any(|o| o.get_number() == TcpOptionNumbers::MSS);

                                                let src_port = tcp.get_source();
                                                let dst_port = tcp.get_destination();

                                                let src = ip4pkt.get_source();
                                                let dst = ip4pkt.get_destination();
                                                trace!("re-transmission request detected: {}:{} -> {}:{}. Connection quality issues?", src,src_port, dst, dst_port);
                                            }

                                            //SYN, SYN-ACK, ACK, u16
                                            if !has_bit(flags, Flags::SYN){//ignore all but SYN+ flag
                                                continue;
                                            }

                                            let src = ip4pkt.get_source();
                                            let dst = ip4pkt.get_destination();
                                            let outbound = net.contains(src);

                                            if !has_bit(flags, Flags::ACK){//SYN flag
                                                data_sender.send(AppData::Syn(   AppTcp::new(src, dst, outbound, Some(Instant::now()), None))).unwrap();
                                            }else{  //SYN-ACK
                                                data_sender.send(AppData::SynAck(AppTcp::new(src, dst, outbound, None, Some(Instant::now())))).unwrap();
                                            }

                                            continue;
                                        }

                                    }
                                    IpNextHeaderProtocols::Udp => {
                                        //println!("udp:{:?}", ip4pkt);
                                        continue
                                    }
                                    IpNextHeaderProtocols::Icmp => {

                                        if let Some(icmp) = IcmpPacket::new(ip4pkt.payload()){


                                            let dst = ip4pkt.get_destination();
                                            if ip != dst{ //only replies back to us
                                                continue;
                                            }

                                            match icmp.get_icmp_type(){
                                                // IcmpTypes::EchoRequest => {
                                                //     let src = ip4pkt.get_source();
                                                //     info!("ICMP-Request {} -> {} [id:{},seq:{},ttl:{}]", src, dst, pkt_id, pkt_seq, ip4pkt.get_ttl());
                                                // }
                                                IcmpTypes::EchoReply => {

                                                    if let Some(echo) = EchoReplyPacket::new(ip4pkt.payload()){

                                                        let src = ip4pkt.get_destination();
                                                        let dst = ip4pkt.get_source();
                                                        let ttl = ip4pkt.get_ttl();

                                                        let pkt_id = echo.get_identifier();
                                                        let pkt_seq =echo.get_sequence_number();

                                                        data_sender.send(
                                                            AppData::IcmpReply(AppIcmp{src, dst, hop:dst, pkt_id, pkt_seq, ttl})
                                                        ).unwrap();
    
                                                    }
                                                }
                                                IcmpTypes::TimeExceeded => {

                                                    if let Some(timeex_pkt) =  TimeExceededPacket::new(ip4pkt.payload()){
                                                        let hop = ip4pkt.get_source();
                                                        let src = ip4pkt.get_destination(); //this ip

                                                        if let Some(ip4_hdr) =  Ipv4Packet::new(timeex_pkt.payload()){

                                                            let dst = ip4_hdr.get_destination(); //intended 
                                                            let ttl = ip4_hdr.get_ttl(); //this is not reliable... will use seq

                                                            if let Some(echoreq_pkt) = EchoRequestPacket::new(ip4_hdr.payload()){

                                                                let pkt_id = echoreq_pkt.get_identifier();
                                                                let pkt_seq =echoreq_pkt.get_sequence_number();

                                                                data_sender.send(AppData::IcmpExceeded(
                                                                    AppIcmp{src, dst, hop, pkt_id, pkt_seq, ttl})
                                                                ).unwrap();

                                                            }
                                                        }
                                                    }
                                                }
                                                IcmpTypes::DestinationUnreachable => {
                                                    //println!("=============> IcmpTypes::DestinationUnreachable {}<=========================", ip4pkt.get_source());

                                                    if let Some(unreach_pkt) =  DestinationUnreachablePacket::new(ip4pkt.payload()){
                                                        let hop = ip4pkt.get_source();
                                                        let src = ip4pkt.get_destination(); //this ip
                                                        if let Some(ip4_hdr) =  Ipv4Packet::new(unreach_pkt.payload()){
                                                            let dst = ip4_hdr.get_destination(); //intended 
                                                            let ttl = ip4_hdr.get_ttl();
                                                            if let Some(echoreq_pkt) = EchoRequestPacket::new(ip4_hdr.payload()){
                                                                let pkt_id = echoreq_pkt.get_identifier();
                                                                let pkt_seq =echoreq_pkt.get_sequence_number();

                                                                data_sender.send(
                                                                    AppData::IcmpUnreachable(AppIcmp{src, dst, hop, pkt_id, pkt_seq, ttl})
                                                                ).unwrap();
                                                            }

                                                        }
                                                    }
                                                }
                                                // IcmpTypes::Traceroute => {
                                                //     println!("=============> IcmpTypes::Traceroute <=========================")
                                                // }
                                                _ => {
                                                    println!("icmp type:{:?}",icmp.get_icmp_type());
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