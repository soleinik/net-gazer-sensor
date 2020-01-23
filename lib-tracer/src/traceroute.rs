use std::net::Ipv4Addr;

use pnet::packet::icmp::{IcmpTypes};

use pnet::packet::MutablePacket;
use pnet::packet::ipv4::MutableIpv4Packet;
use  pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::util;

use lib_data::{AppResult, AppTraceRouteTask};
use async_std::task;

/* 
create_icmp_packet function is stolen from 
    https://codereview.stackexchange.com/questions/208875/traceroute-implementation-in-rust
*/

static IPV4_HEADER_LEN: usize = 21;
static ICMP_HEADER_LEN: usize = 8;
static ICMP_PAYLOAD_LEN: usize = 32;


pub fn create_icmp_packet<'a>( buffer_ip: &'a mut [u8], buffer_icmp: &'a mut [u8], src: Ipv4Addr, dest: Ipv4Addr, id:u16, ttl: u8) -> AppResult<MutableIpv4Packet<'a>> {
    let mut ipv4_packet = MutableIpv4Packet::new(buffer_ip).unwrap();

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(IPV4_HEADER_LEN as u8);
    ipv4_packet.set_total_length((IPV4_HEADER_LEN + ICMP_HEADER_LEN + ICMP_PAYLOAD_LEN) as u16);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);

    ipv4_packet.set_ttl(ttl);

    ipv4_packet.set_source(src);
    ipv4_packet.set_destination(dest);


    /* icmp */
    let mut icmp_packet = MutableEchoRequestPacket::new(buffer_icmp).unwrap();
    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);

    icmp_packet.set_identifier(id);
    icmp_packet.set_sequence_number(ttl as u16);

    
    let checksum = util::checksum(&icmp_packet.packet_mut(), 2);
    icmp_packet.set_checksum(checksum);

    /* set payload */
    ipv4_packet.set_payload(icmp_packet.packet_mut());
    Ok(ipv4_packet)
}



pub fn process(task:AppTraceRouteTask){

    //JoinHandle<T> let child = 
    task::spawn(async move {
        info!("Tracing {} from {} with id:{} and ttl:{}", task.dst, task.src, task.pkt_id, task.ttl + 1);
        //let pkt = create_icmp_packet();
    });

    


}