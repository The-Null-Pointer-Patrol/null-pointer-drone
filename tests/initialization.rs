use std::collections::HashMap;

use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::{DroneCommand, NodeEvent},
    drone::{Drone, DroneOptions},
    packet::{Message, Packet},
};

#[test]
fn test_pdr_outofrange() {
    let (s1, r1) = crossbeam_channel::unbounded::<NodeEvent>();
    let (s2, r2) = crossbeam_channel::unbounded::<DroneCommand>();
    let (s3, r3) = crossbeam_channel::unbounded::<Packet>();
    //let (s4, r4) = crossbeam_channel::unbounded::<Packet>();
    let options = DroneOptions {
        id: 0,
        controller_send: s1,
        controller_recv: r2,
        packet_recv: r3,
        packet_send: HashMap::new(),
        pdr: 145.2,
    };
    let mut d = MyDrone::new(options);
    d.run();
}
