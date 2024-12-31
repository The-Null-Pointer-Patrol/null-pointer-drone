use common::expect::{expect_no_event, expect_panic, try_send_command, try_send_packet};
use common::packetbuilder::PacketBuilder;
use common::{create_channels, start_drone_thread};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use std::collections::HashMap;
use wg_2024::controller::DroneCommand;
use wg_2024::drone::Drone;
use wg_2024::packet::{FloodRequest, Packet, PacketType};

pub mod common;

#[test_log::test]
fn dropped_command_sender() {
    let (es, er, cs, cr, _ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    drop(cs);

    expect_panic(
        handle,
        "The Sender<DroneCommand> end of the simulation controller channel unexpectedly got dropped",
    );
    expect_no_event(&er);
}

#[test_log::test]
fn dropped_packet_sender() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    drop(ps);

    expect_panic(
        handle,
        "There is no connected sender to the drone's packet receiver channel and no DroneCommand::Crash has been received",
    );
    expect_no_event(&er);
}

#[test_log::test]
fn remove_nonexisting_channel() {
    let (es, er, cs, cr, _ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    try_send_command(&cs, DroneCommand::RemoveSender(1));

    expect_panic(handle, "Cannot remove channel to 1: it does not exist");
    expect_no_event(&er);
}

#[test_log::test]
fn empty_routing_header() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![];

    let p = PacketBuilder::new_fragment(hops).build();

    try_send_packet(&ps, p.clone());

    expect_panic(
        handle,
        "empty routing header for packet Packet(0) { routing_header: [  ], pack_type Fragment { index: 1 out of 1, data: 0x0000000000000000000000000000000000000000... + other 108 bytes } }",
    );
    expect_no_event(&er);
}

#[test_log::test]
fn hop_index_out_of_bounds() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![1, 2];

    let p = PacketBuilder::new_fragment(hops.clone())
        .hop_index(2)
        .build();

    try_send_packet(&ps, p.clone());

    expect_panic(handle, "hop_index out of bounds: index 2 for hops [1, 2]");
    expect_no_event(&er);
}

#[test_log::test]
fn hop_index_0() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![1, 2, 3, 4, 5];

    let p = PacketBuilder::new_fragment(hops.clone())
        .hop_index(0)
        .build();

    try_send_packet(&ps, p.clone());

    expect_panic(
        handle,
        "received packet with hop_index 0, which should be impossible",
    );
    expect_no_event(&er);
}

#[test_log::test]
fn no_path_trace() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let senders = HashMap::new();
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let p = PacketBuilder::new(
        PacketType::FloodRequest(FloodRequest {
            flood_id: 0,
            initiator_id: 0,
            path_trace: vec![],
        }),
        vec![],
    )
    .build();

    try_send_packet(&ps, p.clone());

    expect_panic(handle, "flood request has no path trace");
    expect_no_event(&er);
}

#[test_log::test]
fn cant_send_into_channel() {
    let (es, _er, _cs, cr, ps, pr) = create_channels();

    let (s0, _r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![0, 1, 2];

    let p = PacketBuilder::new_fragment(hops.clone()).build();

    drop(r2);
    try_send_packet(&ps, p.clone());

    expect_panic(
        handle,
        "Cannot send packet Packet(0) { routing_header: [ 0 -> 1 ->(2) ], pack_type Fragment { index: 1 out of 1, data: 0x0000000000000000000000000000000000000000... + other 108 bytes } } into channel Sender { .. }. Error: \"SendError(..)\"",
    );
}

#[test_log::test]
fn cant_send_to_sc() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let (s0, _r0) = unbounded::<Packet>();
    let (s2, _r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);
    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![0, 1, 2];

    let p = PacketBuilder::new_fragment(hops.clone()).build();

    drop(er);
    try_send_packet(&ps, p.clone());

    expect_panic(
        handle,
        "Cannot send event PacketSent(Packet { routing_header: SourceRoutingHeader { hop_index: 2, hops: [0, 1, 2] }, session_id: 0, pack_type: MsgFragment(Fragment { fragment_index: 0, total_n_fragments: 1, length: 128, data: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }) }) to simulation controller. Error: \"SendError(..)\"",
    );
}
