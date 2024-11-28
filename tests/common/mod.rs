use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::{DroneCommand, NodeEvent},
    drone::{Drone, DroneOptions},
    packet::Packet,
};

pub fn default_drone() -> (
    DroneOptions,
    Receiver<NodeEvent>,
    Sender<DroneCommand>,
    Sender<Packet>,
) {
    let (s1, r1) = crossbeam_channel::unbounded::<NodeEvent>();
    let (s2, r2) = crossbeam_channel::unbounded::<DroneCommand>();
    let (s3, r3) = crossbeam_channel::unbounded::<Packet>();
    let options = DroneOptions {
        id: 0,
        controller_send: s1,
        controller_recv: r2,
        packet_recv: r3,
        packet_send: HashMap::new(),
        pdr: 0.1,
    };
    (options, r1, s2, s3)
}
