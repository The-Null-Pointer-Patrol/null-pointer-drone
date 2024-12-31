use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    packet::Packet,
};

use super::RECV_WAIT_TIME;

// tries to send given packet and panics if unsuccesful
pub fn try_send_packet(send: &Sender<Packet>, packet: Packet) {
    if let Err(e) = send.send(packet) {
        panic!("error sending packet to drone: {e}")
    };
}

// tries to send given command and panics if unsuccesful
pub fn try_send_command(send: &Sender<DroneCommand>, command: DroneCommand) {
    if let Err(e) = send.send(command) {
        panic!("error sending command to drone: {e}")
    };
}

// checks that the only packet that arrives is the expected one, panics in all other cases
pub fn expect_one_packet(rcv: &Receiver<Packet>, expected: &Packet) {
    expect_packet(rcv, expected);
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(_) => {}
        Ok(got) => {
            panic!("not expecting a second packet, got: {got}");
        }
    };
}

/// panics if there is an error in the channel or if the packet is not the expected one
pub fn expect_packet(rcv: &Receiver<Packet>, expected: &Packet) {
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(e) => {
            panic!("error receiving packet: {e}");
        }
        Ok(got) => {
            assert_eq!(&got, expected);
        }
    };
}

/// panics if it receives a packet
pub fn expect_no_packet(rcv: &Receiver<Packet>) {
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(_) => {}
        Ok(got) => {
            panic!("not expecting any packet, got: {got}");
        }
    };
}

/// panics:
/// - if there is an error in the channel
/// - if the event is not the expected one
/// - if there is more than one event
pub fn expect_one_event(rcv: &Receiver<DroneEvent>, expected: &DroneEvent) {
    expect_event(rcv, expected);
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(_) => {}
        Ok(got) => {
            panic!("was expecting just one event, received also: {got:?}");
        }
    };
}

/// panics if there is an error in the channel or if the event is not the expected one
pub fn expect_event(rcv: &Receiver<DroneEvent>, expected: &DroneEvent) {
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(e) => {
            panic!("error receiving event: {e}");
        }
        Ok(got) => {
            assert_eq!(&got, expected);
        }
    };
}

/// panics if the channel receives anything
pub fn expect_no_event(rcv: &Receiver<DroneEvent>) {
    match rcv.recv_timeout(std::time::Duration::from_millis(RECV_WAIT_TIME)) {
        Err(_) => {}
        Ok(got) => {
            panic!("not expecting a second event, got: {got:?}");
        }
    };
}

pub fn expect_panic<T>(handle: JoinHandle<T>, message: &str) {
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
            assert_eq!(msg, message);
        }
    }
}
