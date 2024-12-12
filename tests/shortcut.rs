use common::expect::{
    expect_event, expect_no_event, expect_no_packet, expect_one_event, expect_one_packet,
    expect_packet, try_send_packet,
};
use common::packetbuilder::PacketBuilder;
use common::{create_channels, default_fragment, start_drone_thread, RECV_WAIT_TIME};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::warn;
use null_pointer_drone::MyDrone;
use std::{collections::HashMap, time::Duration};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Ack, FloodResponse, Nack, NackType, NodeType, Packet, PacketType},
};
mod common;

/// sends a packet:
/// 0 -> (1) -> 2
/// drone 1 has 2 as neighbor but the receiver of 2 has been dropped as would happen in a crash
#[test_log::test]
fn shortcut() {
    let (event_send, event_recv, command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 1.0);
    let handle = start_drone_thread(my_drone);

    // simulate crash
    match command_send.send(DroneCommand::RemoveSender(2)) {
        Ok(_) => {}
        Err(e) => panic!("could not remove sender for drone 2 to simulate crash, err: {e}"),
    };
    drop(r2);

    // --------------------------------------------------------------------------------------------
    // checks for packets that should be shortcutted
    // --------------------------------------------------------------------------------------------
    let hops = vec![0, 1, 2];
    let p1 = PacketBuilder::new_floodresp(hops.clone()).build();
    let p2 = PacketBuilder::new_nack(hops.clone(), NackType::Dropped).build();
    let p3 = PacketBuilder::new_ack(hops.clone()).build();

    for mut p in [p1, p2, p3] {
        try_send_packet(&packet_send, p.clone());

        expect_no_packet(&r0);

        // we expect the packet to be pointing to where it was trying to be sent to, which is the
        // next hop
        p.routing_header.hop_index += 1;
        let expected = DroneEvent::ControllerShortcut(p);

        expect_one_event(&event_recv, expected);
    }

    // --------------------------------------------------------------------------------------------
    // check for packets that just send errorinrouting
    // --------------------------------------------------------------------------------------------
    let p4 = PacketBuilder::new_fragment(hops.clone()).build();

    if let Err(_e) = packet_send.send(p4.clone()) {
        panic!("error sending packet to drone")
    };

    let expected = PacketBuilder::new_nack(vec![1, 0], NackType::ErrorInRouting(2)).build();
    expect_packet(&r0, expected.clone());

    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // in the case of floodRequest the channel has been dropped and removed from neighbors, and
    // floodreq ignores source routing header anyway, so it behaves like a normal flooding, which
    // is already checked in its own test
}
