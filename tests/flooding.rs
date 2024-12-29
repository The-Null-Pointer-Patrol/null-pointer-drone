use std::{collections::HashMap, time::Duration};

use common::{
    create_channels,
    expect::{expect_no_packet, expect_one_event, expect_one_packet, try_send_packet},
    packetbuilder::PacketBuilder,
    start_drone_thread, RECV_WAIT_TIME,
};
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
    let (event_send, event_recv, _controller_send, controller_recv, packet_send, packet_recv) =
        create_channels();

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

/// topology: 0<->1
/// send a flood request with path trace containing just node 0
/// drone #1 is expected to create a floodresponse as it has no other neighbors
#[test]
fn flood_request_no_neighbors() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);

    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    let packet = PacketBuilder::new_floodreq(vec![(0, NodeType::Client)]).build();

    try_send_packet(&ps, packet.clone());

    let expected = PacketBuilder::new_floodresp(
        vec![1, 0],
        vec![(0, NodeType::Client), (1, NodeType::Drone)],
    )
    .build();

    expect_one_packet(&r0, expected.clone());
    expect_one_event(&er, DroneEvent::PacketSent(expected));
}

// TODO: flood_request_id_seen_already

/// topology: 0<->1<->2
/// send a flood request to (1) with path trace containing just node 0
/// confirm that it gets forwarded to (2)
/// send flood request to (1) with path trace containing just node 0 and same flood id as before
/// drone #1 is expected to create a floodresponse as it has already seen the flood request
#[test]
fn flood_request_id_seen_already() {
    let (es, er, _cs, cr, ps, pr) = create_channels();

    let (s0, r0) = unbounded::<Packet>();
    let (s2, r2) = unbounded::<Packet>();
    let mut senders = HashMap::new();
    senders.insert(0, s0);
    senders.insert(2, s2);

    let my_drone = MyDrone::new(1, es, cr, pr, senders, 0.0);
    let _handle = start_drone_thread(my_drone);

    // -------------------------------------------
    // sending request that will be forwarded
    // -------------------------------------------
    let packet = PacketBuilder::new_floodreq(vec![(0, NodeType::Client)]).build();

    try_send_packet(&ps, packet.clone());

    let expected = PacketBuilder::new_floodreq(vec![(0, NodeType::Client), (1, NodeType::Drone)])
        .hop_index(1)
        .hops(vec![1, 2])
        .build();

    expect_no_packet(&r0);
    expect_one_packet(&r2, expected.clone());
    expect_one_event(&er, DroneEvent::PacketSent(expected));

    // -------------------------------------------
    // sending request that will be transformed into a response because drone already received
    // flood id
    // -------------------------------------------
    try_send_packet(&ps, packet.clone());

    let expected = PacketBuilder::new_floodresp(
        vec![1, 0],
        vec![(0, NodeType::Client), (1, NodeType::Drone)],
    )
    .build();

    expect_no_packet(&r2);
    expect_one_packet(&r0, expected.clone());
    expect_one_event(&er, DroneEvent::PacketSent(expected));
}
