use std::{collections::HashMap, error::Error, time::Duration};

use common::{default_drone, default_fragment, start_drone_thread};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use wg_2024::{
    drone::{Drone, DroneOptions},
    network::SourceRoutingHeader,
    packet::{Packet, PacketType},
};
mod common;

#[test]
fn forward_frag() {
    let (def_drone_opts, _recv_event, _send_command, send_packet) = default_drone();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, s2);
    let my_drone = MyDrone::new(DroneOptions {
        packet_send: senders,
        pdr: 0.0,
        ..def_drone_opts
    });
    let handle = start_drone_thread(my_drone);

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

    match r2.recv() {
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
        Ok(packet2) => {
            packet.routing_header.hop_index = 1;
            // todo: enable IF PR gets approved
            //assert_eq!(packet2, packet);
        }
    };
}
