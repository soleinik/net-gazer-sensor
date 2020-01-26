use std::net::Ipv4Addr;

use pnet::packet::icmp::IcmpTypes;

use pnet::packet::{Packet, MutablePacket};
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


fn create_icmp_packet<'a>( buffer_ip: &'a mut [u8], buffer_icmp: &'a mut [u8], src: Ipv4Addr, dest: Ipv4Addr, id:u16, ttl: u8) -> AppResult<MutableIpv4Packet<'a>> {
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

    icmp_packet.set_identifier(id); //host order
    icmp_packet.set_sequence_number(ttl as u16); //host order

    let checksum = util::checksum(icmp_packet.packet(), 1);
    icmp_packet.set_checksum(checksum);

    /* set payload */
    ipv4_packet.set_payload(icmp_packet.packet_mut());
    Ok(ipv4_packet)
}


use pnet::transport::{transport_channel, TransportChannelType::Layer3};

const ICMP_MESSAGE :&[u8] = b"netgazer";

pub fn process(task:AppTraceRouteTask){

    task::spawn(async move {
        debug!("Sending probe {} from {} with id/seq/ttl:{}/{}/{}", task.dst, task.src, task.pkt_id, task.pkt_seq, task.ttl);
        
        //FIXME - reuse buffer. ThreadLocal?
        let mut buffer_ip = [0u8; 40];
        let mut buffer_icmp = [0u8; MutableEchoRequestPacket::minimum_packet_size()];
        buffer_icmp.copy_from_slice(ICMP_MESSAGE);

        let pkt = create_icmp_packet(&mut buffer_ip, &mut buffer_icmp, task.src, task.dst, task.pkt_id, task.ttl).unwrap();

        let protocol = Layer3(IpNextHeaderProtocols::Icmp);

        if let Ok((mut tx, _)) = transport_channel(1024, protocol){

            if tx.send_to(pkt, std::net::IpAddr::V4(task.dst)).is_ok(){
                trace!("pkt sent to {} id:{} seq:{}, ttl:{}",  task.dst, task.pkt_id, task.pkt_seq, task.ttl);
            }else{
                error!("failed to send packet to {}", task.dst);
            }

        }

    });

    


}