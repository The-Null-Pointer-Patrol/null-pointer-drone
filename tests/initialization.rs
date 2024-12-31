use std::collections::HashMap;

use common::create_channels;
use null_pointer_drone::MyDrone;
use wg_2024::{drone::Drone, packet::Packet};

pub mod common;

#[test]
#[should_panic(
    expected = "Tried to set an invalid pdr value of 43.14, which is not in range (0.0..=1.0)"
)]
fn pdr_too_big() {
    let (controller_send, _, _, controller_recv, _packet_send, packet_recv) = create_channels();
    let _my_drone = MyDrone::new(
        0,
        controller_send,
        controller_recv,
        packet_recv,
        HashMap::new(),
        43.14,
    );
}

#[test]
#[should_panic(
    expected = "Tried to set an invalid pdr value of -0.1, which is not in range (0.0..=1.0)"
)]
fn pdr_negative() {
    let (controller_send, _, _, controller_recv, _packet_send, packet_recv) = create_channels();
    let _my_drone = MyDrone::new(
        0,
        controller_send,
        controller_recv,
        packet_recv,
        HashMap::new(),
        -0.1,
    );
}

#[test]
#[should_panic(expected = "Cannot add a channel with the same NodeId of this drone (which is 1)")]
fn neighbor_is_self() {
    let (controller_send, _, _, controller_recv, _packet_send, packet_recv) = create_channels();

    let (sender, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, sender);
    let _my_drone = MyDrone::new(
        1,
        controller_send,
        controller_recv,
        packet_recv,
        senders,
        0.3,
    );
}
