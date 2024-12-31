use std::collections::HashMap;

use common::{
    create_channels,
    expect::{expect_event, expect_no_packet, expect_one_event, expect_one_packet},
    packetbuilder::PacketBuilder,
    start_drone_thread,
};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::DroneEvent,
    drone::Drone,
    packet::{NackType, Packet},
};

pub mod common;

/// sends this packet:
/// 0 -> (1) -> 2
/// but drone 1 has pdr of 1.0 so it gets dropped, and we expect:
/// - a nack of this form: 1->(0)
/// - a PacketSent() with the created nack to sim controller
/// - a Dropped message to sim controller
/// - nothing on drone2 receiver
#[test_log::test]
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

    let packet = PacketBuilder::new_fragment(vec![0, 1, 2]).build();

    // as if we're sending from drone 0
    if let Err(_e) = packet_send.send(packet.clone()) {
        panic!("error sending packet to drone")
    };

    let expected = PacketBuilder::new_nack(vec![1, 0], NackType::Dropped).build();

    expect_one_packet(&r0, expected.clone());

    expect_no_packet(&r2);

    expect_event(&event_recv, DroneEvent::PacketDropped(packet));
    expect_one_event(&event_recv, DroneEvent::PacketSent(expected));
}
