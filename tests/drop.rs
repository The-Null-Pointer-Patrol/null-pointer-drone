use std::{collections::HashMap, time::Duration};

use common::{create_channels, default_fragment, start_drone_thread, RECV_WAIT_TIME};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::warn;
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::DroneEvent,
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Ack, FloodResponse, Nack, NackType, NodeType, Packet, PacketType},
};
mod common;

/// sends this packet:
/// 0 -> (1) -> 2
/// but drone 1 has pdr of 1.0 so it gets dropped, and we expect:
/// - a nack of this form: 1->(0)
/// - a Dropped message to sim controller
/// - nothing on drone2 receiver
#[test]
fn expect_drop() {
    let (event_send, event_recv, _controller_send, controller_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);

    let my_drone = MyDrone::new(1, event_send, controller_recv, packet_recv, senders, 1.0);
    let _handle = start_drone_thread(my_drone);

    let mut packet = Packet {
        pack_type: PacketType::MsgFragment(default_fragment(4, 10)),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };

    // as if we're sending from drone 0
    if let Err(_e) = packet_send.send(packet.clone()) {
        panic!("error sending packet to drone")
    };

    let expected = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 4,
            nack_type: NackType::Dropped,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 0],
        },
        session_id: 100,
    };

    match r0.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
        Ok(p2) => {
            assert_eq!(p2, expected);
        }
    };
    match r2.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
        Err(_) => {}
        Ok(p2) => {
            panic!("no packet should arrive to drone 2, got packet {}", p2);
        }
    };
    match event_recv.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
        Ok(e2) => {
            packet.routing_header.increase_hop_index();
            let expected = DroneEvent::PacketDropped(packet.clone());
            assert_eq!(e2, expected);
        }
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
    }
}
