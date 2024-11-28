use std::collections::HashMap;

use common::default_drone;
use null_pointer_drone::MyDrone;
use wg_2024::{
    drone::{Drone, DroneOptions},
    packet::Packet,
};
mod common;

#[test]
#[should_panic(expected = "pdr out of bounds")]
fn pdr_outofrange() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();
    let _my_drone = MyDrone::new(DroneOptions {
        pdr: 345.3,
        ..def_drone_opts
    });
}

#[test]
#[should_panic(expected = "neighbor has id 1 which is the same as drone")]
fn neighbor_is_self() {
    let (def_drone_opts, _recv_event, _send_command, _send_packet) = default_drone();

    let (sender, _) = crossbeam_channel::unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, sender);
    let _my_drone = MyDrone::new(DroneOptions {
        packet_send: senders,
        id: 1,

        ..def_drone_opts
    });
}
