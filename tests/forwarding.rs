use std::collections::HashMap;

use common::{
    create_channels,
    expect::{expect_no_packet, expect_one_event, expect_one_packet, try_send_packet},
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

#[test_log::test]
fn forward() {
    let (event_send, event_recv, _command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    let hops = vec![0, 1, 2, 3, 4];

    let p1 = PacketBuilder::new_nack(hops.clone(), NackType::Dropped).build();
    let p2 = PacketBuilder::new_nack(hops.clone(), NackType::DestinationIsDrone).build();
    let p3 = PacketBuilder::new_nack(hops.clone(), NackType::ErrorInRouting(14)).build();
    let p4 = PacketBuilder::new_nack(hops.clone(), NackType::UnexpectedRecipient(34)).build();
    let p5 = PacketBuilder::new_ack(hops.clone()).build();
    let p6 = PacketBuilder::new_fragment(hops.clone()).build();
    let p7 = PacketBuilder::new_floodresp(hops.clone(), vec![]).build();

    for mut p in [p1, p2, p3, p4, p5, p6, p7] {
        try_send_packet(&packet_send, p.clone());

        p.routing_header.hop_index += 1;
        expect_one_packet(&r2, &p);
        expect_no_packet(&r0);
        expect_one_event(&event_recv, &DroneEvent::PacketSent(p));
    }
}
