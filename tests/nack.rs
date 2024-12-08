use crossbeam_channel::unbounded;
use null_pointer_drone::MyDrone;
use std::{collections::HashMap, thread};
use wg_2024::{
    controller::{DroneCommand, DroneEvent},
    drone::Drone,
    network::SourceRoutingHeader,
    packet::{Fragment, Nack, NackType, Packet, PacketType},
};
mod common;

#[test]
fn mismatched_hop_index() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send, neighbour_receive) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send);

    // Create drone
    let mut drone = MyDrone::new(
        3, // arbitrary choice
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    // Invalid header with mismatched hop index and drone position
    let source_routing_header = SourceRoutingHeader {
        hop_index: 2,                 // the hop index is mismatched with drone id
        hops: vec![0, 1, 2, 3, 4, 5], // drone is set to be destination
    };

    // Nothing special fragment
    let fragment = Fragment {
        fragment_index: 0,
        total_n_fragments: 1,
        length: 128,
        data: [0; 128],
    };

    // Wrap fragment and source route header to a packet
    let packet = Packet {
        pack_type: PacketType::MsgFragment(fragment),
        routing_header: source_routing_header,
        session_id: 1,
    };

    // Send the packet
    assert!(
        packet_send.send(packet.clone()).is_ok(),
        "Failed to send packet."
    );

    // Read the output from receiver's incoming channel
    match neighbour_receive.recv() {
        Err(error) => {
            panic!("Failed to receive the packet: {error}");
        }
        Ok(received_packet) => {
            // Construct the expected packet
            let expected_routing_header = SourceRoutingHeader {
                hop_index: 1,
                hops: vec![2, 1, 0], // order should be reversed from the drone
            };

            let expected_nack = Nack {
                fragment_index: 0,
                // mismatched hop index and drone id
                nack_type: NackType::UnexpectedRecipient(3),
            };

            let expected_packet = Packet {
                pack_type: PacketType::Nack(expected_nack),
                routing_header: expected_routing_header,
                session_id: 1, // should remain the same
            };

            assert_eq!(received_packet, expected_packet);
        }
    };
}

#[test]
fn drone_as_destination() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send, neighbour_receive) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send);

    // Create drone
    let mut drone = MyDrone::new(
        2, // will be set as last node in hops
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    // Header with invalid data as drone is set to be destination
    let source_routing_header = SourceRoutingHeader {
        hop_index: 2,        // set hop index to match drone id
        hops: vec![0, 1, 2], // drone is set to be destination
    };

    // Nothing-special-fragment
    let fragment = Fragment {
        fragment_index: 0,
        total_n_fragments: 1,
        length: 128,
        data: [0; 128],
    };

    // Wrap fragment and source route header to a packet
    let packet = Packet {
        pack_type: PacketType::MsgFragment(fragment),
        routing_header: source_routing_header,
        session_id: 1,
    };

    // Send the packet
    assert!(
        packet_send.send(packet.clone()).is_ok(),
        "Failed to send packet."
    );

    // Read the output from receiver's incoming channel
    match neighbour_receive.recv() {
        Err(error) => {
            panic!("Failed to receive the packet: {error}");
        }
        Ok(received_packet) => {
            // Construct the expected packet
            let expected_routing_header = SourceRoutingHeader {
                hop_index: 1,
                hops: vec![2, 1, 0], // order should be reversed from the original
            };

            let expected_nack = Nack {
                fragment_index: 0,
                nack_type: NackType::DestinationIsDrone,
            };

            let expected_packet = Packet {
                pack_type: PacketType::Nack(expected_nack),
                routing_header: expected_routing_header,
                session_id: 1, // should remain the same
            };

            assert_eq!(received_packet, expected_packet);
        }
    };
}

#[test]
fn nack_when_dropped() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send_1, neighbour_receive_1) = unbounded::<Packet>();
    let (neighbour_send_3, _neighbour_receive_3) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send_1);
    neighbors.insert(3, neighbour_send_3);

    // Create drone
    let mut drone = MyDrone::new(
        2, // arbitrary choice
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        1.0, // ensure that packet will be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    // Nothing-special-header
    let source_routing_header = SourceRoutingHeader {
        hop_index: 2, // set hop index to match drone id
        hops: vec![0, 1, 2, 3],
    };

    // Nothing-special-fragment
    let fragment = Fragment {
        fragment_index: 0,
        total_n_fragments: 1,
        length: 128,
        data: [0; 128],
    };

    // Wrap fragment and source route header to a packet
    let packet = Packet {
        pack_type: PacketType::MsgFragment(fragment),
        routing_header: source_routing_header,
        session_id: 1,
    };

    // Send the packet
    assert!(
        packet_send.send(packet.clone()).is_ok(),
        "Failed to send packet."
    );

    // Read the output from receiver's incoming channel
    match neighbour_receive_1.recv() {
        Err(error) => {
            panic!("Failed to receive the packet: {error}");
        }
        Ok(received_packet) => {
            // Construct the expected packet
            let expected_routing_header = SourceRoutingHeader {
                hop_index: 1,
                hops: vec![2, 1, 0], // order should be reversed from the original
            };

            let expected_nack = Nack {
                fragment_index: 0,
                nack_type: NackType::Dropped, // packet should be dropped
            };

            let expected_packet = Packet {
                pack_type: PacketType::Nack(expected_nack),
                routing_header: expected_routing_header,
                session_id: 1, // should remain the same
            };

            assert_eq!(received_packet, expected_packet);
        }
    };
}

#[test]
fn nack_when_there_are_no_neighbours() {
    // Create channels
    let (packet_send, packet_recv) = crossbeam_channel::unbounded::<Packet>();
    let (controller_send, _) = crossbeam_channel::unbounded::<DroneEvent>();
    let (_, controller_recv) = crossbeam_channel::unbounded::<DroneCommand>();

    // create neighbor
    let (neighbour_send_1, neighbour_receive_1) = unbounded::<Packet>();
    let mut neighbors = HashMap::new();
    neighbors.insert(1, neighbour_send_1);

    // Create drone
    let mut drone = MyDrone::new(
        2, // arbitrary choice
        controller_send,
        controller_recv,
        packet_recv,
        neighbors,
        0.0, // ensure that packet will not be dropped
    );

    // Run drone
    thread::spawn(move || {
        drone.run();
    });

    // Header with invalid hops
    let source_routing_header = SourceRoutingHeader {
        hop_index: 2,           // set hop index to match drone id
        hops: vec![0, 1, 2, 3], // drone does not know about drone id 3
    };

    // Nothing-special-fragment
    let fragment = Fragment {
        fragment_index: 0,
        total_n_fragments: 1,
        length: 128,
        data: [0; 128],
    };

    // Wrap fragment and source route header to a packet
    let packet = Packet {
        pack_type: PacketType::MsgFragment(fragment),
        routing_header: source_routing_header,
        session_id: 1,
    };

    // Send the packet
    assert!(
        packet_send.send(packet.clone()).is_ok(),
        "Failed to send packet."
    );

    // Read the output from receiver's incoming channel
    match neighbour_receive_1.recv() {
        Err(error) => {
            panic!("Failed to receive the packet: {error}");
        }
        Ok(received_packet) => {
            // Construct the expected packet
            let expected_routing_header = SourceRoutingHeader {
                hop_index: 1,
                hops: vec![2, 1, 0], // order should be reversed from the original
            };

            let expected_nack = Nack {
                fragment_index: 0,
                nack_type: NackType::ErrorInRouting(3), // neighbor not known
            };

            let expected_packet = Packet {
                pack_type: PacketType::Nack(expected_nack),
                routing_header: expected_routing_header,
                session_id: 1, // should remain the same
            };

            assert_eq!(received_packet, expected_packet);
        }
    };
}
