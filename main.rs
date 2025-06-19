use pnet::datalink::{self, Channel::Ethernet};
use pnet::packet::{Packet};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use tokio::sync::SetError;
use std::net::Ipv4Addr;
use std::sync::Arc;
use reqwest;
use regex::Regex;

#[tokio::main]
async fn main() {
    let list = request(
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(192, 168, 1, 255),
    );
    for mac in list{

    let link =  format!(r"https://www.macvendorlookup.com/api/v2/{{{}}}",mac)
    let client = reqwest::Client::new();
    let response = client
    .get(link)
    .send()
    .await
    .unwrap()
    .text()
    .await;
    }



    let re = Regex::new(r#""company"\s*:\s*"([^"]+)""#).unwrap();
    if let Some(caps ) = re.captures(&response.unwrap().as_str().clone()){

        println!("Company used : {}",&caps[1]);
    }





    println!("{:?}",list);








}

fn request(start_ip: Ipv4Addr, end_ip: Ipv4Addr) -> Vec<String>{
    let mut list_macs:Vec<String> = vec![];
    let interface_name = "en0"; // Change this based on your system (use `ifconfig` to check)
    let interfaces = datalink::interfaces();
    let interface = interfaces.into_iter()
        .find(|iface| iface.name == interface_name)
        .expect("Interface not found");

    let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("Failed to create datalink channel: {}", e),
    };

    let source_mac = interface.mac.expect("Interface has no MAC address");
    let source_ip = Ipv4Addr::new(192, 168, 1, 100); // Your own IP (adjust if needed)

    for target_ip in ip_range(start_ip, end_ip) {
        let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
        ethernet_packet.set_destination(pnet::datalink::MacAddr::broadcast());
        ethernet_packet.set_source(source_mac);
        ethernet_packet.set_ethertype(EtherTypes::Arp);

        let mut arp_buffer = [0u8; 28];
        {
            let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();
            arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
            arp_packet.set_protocol_type(EtherTypes::Ipv4);
            arp_packet.set_hw_addr_len(6);
            arp_packet.set_proto_addr_len(4);
            arp_packet.set_operation(ArpOperations::Request);
            arp_packet.set_sender_hw_addr(source_mac);
            arp_packet.set_sender_proto_addr(source_ip);
            arp_packet.set_target_hw_addr(pnet::datalink::MacAddr::zero());
            arp_packet.set_target_proto_addr(target_ip);
        }

        ethernet_packet.set_payload(&arp_buffer);
        match tx.send_to(ethernet_packet.packet(), None) {
            Some(Ok(_)) => {} // success
            Some(Err(e)) => {
                eprintln!("Failed to send ARP request to {}: {}", target_ip, e);
                continue;
            }
            None => {
                eprintln!("Failed to send ARP request to {}: buffer unavailable", target_ip);
                continue;
            }
        }
        // Listen for a reply to this specific IP
        let mut found = false;
        for _ in 0..10 { // Try up to 10 packets before giving up
            let packet = match rx.next() {
                Ok(pkt) => pkt,
                Err(_) => continue,
            };
            if let Some(ethernet_packet) = EthernetPacket::new(packet) {
                if ethernet_packet.get_ethertype() == EtherTypes::Arp {
                    if let Some(arp_packet) = ArpPacket::new(ethernet_packet.payload()) {
                        if arp_packet.get_operation() == ArpOperations::Reply &&
                            arp_packet.get_target_proto_addr() == source_ip &&
                            arp_packet.get_sender_proto_addr() == target_ip {
                            //println!(
                               // "{}",
                              //  arp_packet.get_sender_hw_addr()
                            //);
                            list_macs.push(arp_packet.get_sender_hw_addr().to_string());
                            found = true;
                            break;
                        }
                    }
                }
            }
        }


    }
    list_macs
}

// Converts a range of IPs into a Vec<Ipv4Addr>

fn ip_range(start: Ipv4Addr, end: Ipv4Addr) -> Vec<Ipv4Addr> {
    let start_u32 = u32::from(start);
    let end_u32 = u32::from(end);
    (start_u32..=end_u32).map(Ipv4Addr::from).collect()
}






