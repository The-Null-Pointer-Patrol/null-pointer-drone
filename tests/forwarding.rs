use std::{collections::HashMap, time::Duration};

use common::{default_drone, default_fragment};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use wg_2024::{
    drone::{Drone, DroneOptions},
    network::SourceRoutingHeader,
    packet::{Fragment, Packet, PacketType},
};
mod common;

#[test]
fn forward_frag() {
    let (def_drone_opts, _recv_event, _send_command, send_packet) = default_drone();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, s2);
    let _my_drone = MyDrone::new(DroneOptions {
        packet_send: senders,
        ..def_drone_opts
    });

    let frag = default_fragment(0, 10);

    let mut packet = Packet {
        pack_type: PacketType::MsgFragment(frag),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    if let Err(e) = send_packet.send(packet.clone()) {
        panic!("error sending packet to drone")
    };
    match r2.recv_timeout(Duration::from_millis(200)) {
        Err(_) => {
            panic!("timeout receiving packet from drone")
        }
        Ok(packet2) => {
            packet.routing_header.hop_index = 1;
            // todo: enable when PR gets approved
            //assert_eq!(packet2, packet);
        }
    };
}
