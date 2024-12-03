use std::collections::HashMap;

use common::create_channels;
use null_pointer_drone::MyDrone;
use wg_2024::{drone::Drone, packet::Packet};

mod common;

#[test]
#[should_panic(expected = "pdr out of bounds")]
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
#[should_panic(expected = "pdr out of bounds")]
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
#[should_panic(expected = "neighbor with id 1 which is the same as drone")]
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
