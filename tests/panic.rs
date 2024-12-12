use common::expect::{expect_no_event, try_send_packet};
use common::packetbuilder::PacketBuilder;
use common::{create_channels, start_drone_thread};
use null_pointer_drone::MyDrone;
use std::collections::HashMap;
use wg_2024::drone::Drone;
mod common;

#[test_log::test]
fn panic_1() {
    let (event_send, event_recv, _command_send, command_recv, packet_send, packet_recv) =
        create_channels();

    let senders = HashMap::new();

    let my_drone = MyDrone::new(1, event_send, command_recv, packet_recv, senders, 0.0);
    let handle = start_drone_thread(my_drone);

    let hops = vec![1, 2, 3, 4, 5];

    let p = PacketBuilder::new_fragment(hops.clone())
        .hop_index(0)
        .build();

    try_send_packet(&packet_send, p.clone());

    // check that the drone thread panicked with the correct error message
    match handle.join() {
        Ok(_) => {
            panic!("Drone did not panic when sending packet with hop_index 0")
        }
        Err(err) => {
            let msg = match err.downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match err.downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => panic!("could not extract error message from joined thread"),
                },
            };
            assert_eq!(
                msg,
                "received packet with hop_index 0, which should be impossible"
            );
        }
    }

    expect_no_event(&event_recv);
}
