use std::{collections::HashMap, time::Duration};

use common::{
    create_channels,
    expect::{
        expect_no_packet, expect_one_event, expect_one_packet, expect_packet, try_send_packet,
    },
    packetbuilder::PacketBuilder,
    start_drone_thread, RECV_WAIT_TIME,
};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    packet::{NackType, Packet},
};

mod common;

/// topology: 0-1-2
/// send 0->(1)->2 and check it forwards
/// remove from 1 sender for 2
/// send 0->(1)->2 and check it returns the correct nack
#[test_log::test]
fn removesender() {
    let (event_send, event_recv, command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);
    let hops = vec![0, 1, 2];

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    // check normal functioning
    let p = PacketBuilder::new_fragment(hops.clone()).build();
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();
    expect_one_packet(&r2, expected.clone());
    expect_no_packet(&r0);
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // remove sender
    match command_send.send(DroneCommand::RemoveSender(2)) {
        Ok(_) => {}
        Err(e) => panic!("could not remove sender for drone 2, err: {e}"),
    };
    drop(r2);

    // check drone behaves as expected when sender is removed
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_nack(vec![1, 0], NackType::ErrorInRouting(2)).build();
    expect_packet(&r0, expected.clone());
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);
}

/// topology: 0-1-2
/// send 0->(1)->2 and check it gives nack
/// add to 1 sender for 2
/// send 0->(1)->2 and check it behaves normally
#[test_log::test]
fn addsender() {
    let (event_send, event_recv, command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    // do not insert sender for 2 yet
    let hops = vec![0, 1, 2];
    let p = PacketBuilder::new_fragment(hops.clone()).build();

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    // check drone gives nack before adding
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_nack(vec![1, 0], NackType::ErrorInRouting(2)).build();
    expect_packet(&r0, expected.clone());
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // add sender
    match command_send.send(DroneCommand::AddSender(2, s2)) {
        Ok(_) => {}
        Err(e) => panic!("could not add sender for drone 2, err: {e}"),
    };

    // check normal functioning
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();
    expect_one_packet(&r2, expected.clone());
    expect_no_packet(&r0);
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);
}

/// topology: 0-1 2
/// send 0->(1)->2 and check it behaves normally
/// set pdr with command to 1.0
/// send 0->(1)->2 and check it gives dropped nack
#[test_log::test]
fn changepdr() {
    let (event_send, event_recv, command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);
    let hops = vec![0, 1, 2];

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    let p = PacketBuilder::new_fragment(hops.clone()).build();

    // check normal functioning
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();
    expect_one_packet(&r2, expected.clone());
    expect_no_packet(&r0);
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // set pdr to 1.0
    match command_send.send(DroneCommand::SetPacketDropRate(1.0)) {
        Ok(_) => {}
        Err(e) => panic!("could not change pdr of drone 1, err: {e}"),
    };

    // check drone gives nack before adding
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_nack(vec![1, 0], NackType::Dropped).build();
    expect_packet(&r0, expected.clone());
}

/// topology: 0-1-2
/// send 0->(1)->2 and check it behaves normally
/// send crash command to drone 1
/// send 0->(1)->2 and check it behaves normally
/// drop senders for drone 1
/// check that the drone thread terminates
#[test_log::test]
fn crash() {
    let (event_send, event_recv, command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);
    let hops = vec![0, 1, 2];

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let p = PacketBuilder::new_fragment(hops.clone()).build();

    // check normal functioning
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();
    expect_one_packet(&r2, expected.clone());
    expect_no_packet(&r0);
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // send crash command
    match command_send.send(DroneCommand::Crash) {
        Ok(_) => {}
        Err(e) => panic!("could not crash drone 1, err: {e}"),
    };

    // check normal functioning
    try_send_packet(&packet_send, p.clone());
    let expected = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();
    expect_one_packet(&r2, expected.clone());
    expect_no_packet(&r0);
    let expected = DroneEvent::PacketSent(expected);
    expect_one_event(&event_recv, expected);

    // drop all packet senders for the drone
    drop(packet_send);

    std::thread::sleep(Duration::from_millis(RECV_WAIT_TIME));

    // check that drone thread returns
    assert!(handle.is_finished());
}
