use std::net::Ipv4Addr;

use pnet::packet::icmp::{IcmpTypes};
use packet_builder::ipv4;
use packet_builder::payload::PayloadData;
use packet_builder::L4Checksum;

use pnet::packet::Packet;
use pnet::util::MacAddr;
use pnet::packet::ethernet::MutableEthernetPacket;

use std::cell::RefCell;
thread_local! {
    pub static PKT_BUF: RefCell<[u8; 1500]> = RefCell::new([0u8; 1500]);
}


// pub fn echo_request<'a>(my_mac:MacAddr, src_ip:Ipv4Addr, dst_ip:Ipv4Addr) -> MutableEthernetPacket<'a>{
//     PKT_BUF.with(| buf| {
//         let mut buf = *buf.borrow_mut();
//         packet_builder!(
//             buf,
//             ether({set_destination => MacAddr(255,255,255,255,255,255), set_source => my_mac}) / 
//             ipv4({set_source => src_ip, set_destination => dst_ip }) /
//             icmp_echo_req({set_icmp_type => IcmpTypes::EchoRequest}) / 
//             payload({"hello".to_string().into_bytes()
//         })
//         )
//     })
// }




// let mut pkt_buf = [0u8; 1500];
// let pkt = packet_builder!(
//      pkt_buf,
//      ether({set_destination => MacAddr(1,2,3,4,5,6), set_source => MacAddr(10,1,1,1,1,1)}) / 
//      ipv4({set_source => ipv4addr!("127.0.0.1"), set_destination => ipv4addr!("127.0.0.1") }) /
//      icmp_echo_req({set_icmp_type => IcmpTypes::EchoRequest}) / 
//      payload({"hello".to_string().into_bytes()})
// );

// sender.send_to(pkt.packet(), None).unwrap().unwrap();