use std::{
    collections::HashMap,
    thread::{self, spawn, JoinHandle},
};
pub const RECV_WAIT_TIME: u64 = 200;
use crossbeam_channel::{Receiver, Sender};
use null_pointer_drone::MyDrone;
use wg_2024::controller::DroneEvent;
use wg_2024::network::NodeId;
use wg_2024::{
    controller::DroneCommand,
    drone::Drone,
    packet::{Fragment, Packet},
};

pub fn create_channels() -> (
    Sender<DroneEvent>,
    Receiver<DroneEvent>,
    Sender<DroneCommand>,
    Receiver<DroneCommand>,
    Sender<Packet>,
    Receiver<Packet>,
) {
    let (s1, r1) = crossbeam_channel::unbounded::<DroneEvent>();
    let (s2, r2) = crossbeam_channel::unbounded::<DroneCommand>();
    let (s3, r3) = crossbeam_channel::unbounded::<Packet>();
    (s1, r1, s2, r2, s3, r3)
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
