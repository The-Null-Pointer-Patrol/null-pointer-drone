use std::{collections::HashMap, time::Duration};

use common::{create_channels, default_fragment, start_drone_thread};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::warn;
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::DroneEvent,
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Ack, Nack, NackType, Packet, PacketType},
};
mod common;

#[test]
fn forward_frag() {
    let (my_drone, packet_send, packet_recv, event_recv) = make_forwarding_drone();
    let _handle = start_drone_thread(my_drone);

    let frag = default_fragment(0, 10);

    let packet = Packet {
        pack_type: PacketType::MsgFragment(frag),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(packet_send, packet_recv, event_recv, packet);
}

#[test]
fn forward_ack() {
    let (my_drone, packet_send, packet_recv, event_recv) = make_forwarding_drone();
    let _handle = start_drone_thread(my_drone);

    let packet = Packet {
        pack_type: PacketType::Ack(Ack { fragment_index: 12 }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(packet_send, packet_recv, event_recv, packet);
}

#[test]
fn forward_nack() {
    let (my_drone, packet_send, packet_recv, event_recv) = make_forwarding_drone();
    let _handle = start_drone_thread(my_drone);

    let p1 = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 12,
            nack_type: NackType::Dropped,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(
        packet_send.clone(),
        packet_recv.clone(),
        event_recv.clone(),
        p1,
    );

    let p2 = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 12,
            nack_type: NackType::ErrorInRouting(3),
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(
        packet_send.clone(),
        packet_recv.clone(),
        event_recv.clone(),
        p2,
    );

    let p3 = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 12,
            nack_type: NackType::UnexpectedRecipient(3),
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(
        packet_send.clone(),
        packet_recv.clone(),
        event_recv.clone(),
        p3,
    );

    let p4 = Packet {
        pack_type: PacketType::Nack(Nack {
            fragment_index: 12,
            nack_type: NackType::DestinationIsDrone,
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };
    send_and_check_forward(packet_send, packet_recv, event_recv, p4);
}

fn make_forwarding_drone() -> (
    MyDrone,
    Sender<Packet>,
    Receiver<Packet>,
    Receiver<DroneEvent>,
) {
    let (controller_send, r1, _, controller_recv, packet_send, packet_recv) = create_channels();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(1, s2);
    let my_drone = MyDrone::new(
        0,
        controller_send,
        controller_recv,
        packet_recv,
        senders,
        0.0,
    );
    (my_drone, packet_send, r2, r1)
}
fn send_and_check_forward(
    s: Sender<Packet>,
    r: Receiver<Packet>,
    er: Receiver<DroneEvent>,
    mut p: Packet,
) {
    if let Err(_e) = s.send(p.clone()) {
        panic!("error sending packet to drone")
    };

    match r.recv() {
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
        Ok(p2) => {
            p.routing_header.hop_index = 1;
            // todo: enable IF PR gets approved
            assert_eq!(p2, p);
        }
    };

    match er.recv() {
        Ok(e2) => {
            let expected = DroneEvent::PacketSent(p);
            assert_eq!(e2, expected);
        }
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
    }
}
