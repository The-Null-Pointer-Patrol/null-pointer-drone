use std::{collections::HashMap, time::Duration};

use common::{create_channels, start_drone_thread, RECV_WAIT_TIME};
use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use wg_2024::{
    controller::DroneEvent,
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{FloodRequest, FloodResponse, NodeType, Packet, PacketType},
};

mod common;

#[test]
fn flood_request_propagation() {
    let (event_send, event_recv, _controller_send, controller_recv, packet_send, packet_recv) = create_channels();

    let (s2, r2) = unbounded::<Packet>();
    let (s3, r3) = unbounded::<Packet>();
    let (s4, r4) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(2, s2);
    senders.insert(3, s3);
    senders.insert(4, s4);

    let my_drone = MyDrone::new(1, event_send, controller_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    let mut packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 1,
            initiator_id: 100,
            path_trace: vec![(100, NodeType::Client)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![0, 1, 2, 3, 4],
        },
        session_id: 100,
    };

    if let Err(_e) = packet_send.send(packet.clone()) {
        panic!("error sending packet to drone")
    };

    // change packet to expected
    //packet.routing_header.hop_index = 1;
    //packet.routing_header.hops = vec![];
    packet.pack_type = PacketType::FloodRequest(FloodRequest {
        flood_id: 1,
        initiator_id: 100,
        path_trace: vec![(100, NodeType::Client), (1, NodeType::Drone)],
    });
    let mut x = 2;
    for r in [r2, r3, r4] {
        // flood request does not care about routing header, but for how it's implemented at the
        // moment a vec of len 2 is created, see process_flood_request for more info
        //packet.routing_header.hops = vec![1, x];
        //x += 1;

        match r.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
            Err(e) => {
                panic!("error receiving packet: {}", e);
            }
            Ok(p2) => {
                assert_eq!(p2.session_id, packet.session_id);
                assert_eq!(p2.pack_type, packet.pack_type);
            }
        };
        match event_recv.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
            Ok(e2) => {
                let expected = DroneEvent::PacketSent(packet.clone());
                match e2 {
                    DroneEvent::PacketSent(p2) => {
                        assert_eq!(p2.session_id, packet.session_id);
                        assert_eq!(p2.pack_type, packet.pack_type);
                    }
                    _ => {
                        panic!("Was expecting event of type PacketsSent, got {:?}", e2)
                    }
                }
            }
            Err(e) => {
                panic!("error receiving packet: {}", e);
            }
        }
    }
}

#[test]
fn flood_request_no_neighbors() {
    let (event_send, event_recv, _controller_send, controller_recv, packet_send, packet_recv) = create_channels();

    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(100, s2);

    let my_drone = MyDrone::new(1, event_send, controller_recv, packet_recv, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    let packet = Packet {
        pack_type: PacketType::FloodRequest(FloodRequest {
            flood_id: 1,
            initiator_id: 100,
            path_trace: vec![(100, NodeType::Drone)],
        }),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: vec![],
        },
        session_id: 100,
    };

    if let Err(_e) = packet_send.send(packet.clone()) {
        panic!("error sending packet to drone")
    };

    let expected = Packet {
        pack_type: PacketType::FloodResponse(FloodResponse {
            flood_id: 1,
            path_trace: vec![(100, NodeType::Drone), (1, NodeType::Drone)],
        }),

        routing_header: SourceRoutingHeader {
            hop_index: 1,
            hops: vec![1, 100],
        },
        session_id: 100,
    };

    match r2.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
        Ok(p2) => {
            // todo: enable IF PR gets approved
            assert_eq!(p2, expected);
        }
    };
    match event_recv.recv_timeout(Duration::from_millis(RECV_WAIT_TIME)) {
        Ok(e2) => {
            let expected = DroneEvent::PacketSent(expected.clone());
            assert_eq!(e2, expected);
        }
        Err(e) => {
            panic!("error receiving packet: {}", e);
        }
    }
}

// TODO: flood_request_id_seen_already
