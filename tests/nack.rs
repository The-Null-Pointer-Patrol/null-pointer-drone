use common::{
    expect::{expect_one_packet, try_send_packet},
    packetbuilder::PacketBuilder,
};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use std::{collections::HashMap, thread};
use test_log::test;
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    packet::{NackType, Packet},
};
mod common;

#[test]
fn mismatched_hop_index() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _cr) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_cs, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send, neighbour_receive) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send);

    // Create drone
    let mut drone = MyDrone::new(
        3, // arbitrary choice
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    let packet = PacketBuilder::new_fragment(vec![0, 1, 2, 3, 4, 5])
        .hop_index(2)
        .build();

    // Send the packet
    try_send_packet(&packet_send, packet.clone());

    let expected_packet =
        PacketBuilder::new_nack(vec![2, 1, 0], NackType::UnexpectedRecipient(3)).build();
    expect_one_packet(&neighbour_receive, expected_packet);
}

#[test]
fn drone_as_destination() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _cr) = crossbeam_channel::unbounded::<DroneEvent>();
    let (cs_, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send, neighbour_receive) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send);

    // Create drone
    let mut drone = MyDrone::new(
        2, // will be set as last node in hops
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    // Wrap fragment and source route header to a packet
    let packet = PacketBuilder::new_fragment(vec![0, 1, 2])
        .hop_index(2)
        .build();

    // Send the packet
    try_send_packet(&packet_send, packet.clone());

    // Read the output from receiver's incoming channel
    let expected_packet =
        PacketBuilder::new_nack(vec![2, 1, 0], NackType::DestinationIsDrone).build();
    expect_one_packet(&neighbour_receive, expected_packet);
}

#[test]
fn nack_when_there_are_no_neighbours() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _cr) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_cs, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send_1, neighbour_receive_1) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send_1);

    // Create drone
    let mut drone = MyDrone::new(
        2, // arbitrary choice
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    let packet = PacketBuilder::new_fragment(vec![0, 1, 2, 3])
        .hop_index(2)
        .build();

    try_send_packet(&packet_send, packet.clone());

    let expected = PacketBuilder::new_nack(vec![2, 1, 0], NackType::ErrorInRouting(3)).build();

    expect_one_packet(&neighbour_receive_1, expected);
}
