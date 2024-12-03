use std::{
    collections::HashMap,
    thread::{self, spawn, JoinHandle},
};

use crossbeam_channel::{Receiver, Sender};
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::DroneCommand,
    drone::Drone,
    packet::{Fragment, Packet},
};
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;

pub fn default_drone() -> (
    Sender<DroneEvent>,
    Receiver<DroneCommand>,
    Receiver<Packet>,
    Sender<Packet>,
) {
    let (s1, r1) = crossbeam_channel::unbounded::<DroneEvent>();
    let (s2, r2) = crossbeam_channel::unbounded::<DroneCommand>();
    let (s3, r3) = crossbeam_channel::unbounded::<Packet>();
    (s1, r2, r3, s3)
}

pub fn default_fragment(idx: u64, n_frags: u64) -> Fragment {
    Fragment {
        fragment_index: idx,
        total_n_fragments: n_frags,
        length: 80,
        data: [0; 128],
    }
}

pub fn start_drone_thread(mut d: MyDrone) -> JoinHandle<()> {
    spawn(move || {
        d.run();
    })
}
