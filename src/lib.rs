use core::panic;
use crossbeam_channel::{select, Receiver, Sender};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use rand::rngs::ThreadRng;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

enum State {
    Working,
    Crashing,
}

pub struct MyDrone {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    known_flood_ids: HashSet<u64>, // TODO: use this field
    state: State,
    // rng: ThreadRng,
}

impl Drone for MyDrone {
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        // TODO: decide if we need more input validation
        assert!(!packet_send.contains_key(&id), "neighbor with id {id} which is the same as drone");
        assert!((0.0..=1.0).contains(&pdr), "pdr out of bounds");
        Self {
            id,
            controller_send,
            controller_recv,
            packet_recv,
            pdr,
            packet_send,
            known_flood_ids: HashSet::new(),
            state: State::Working,
            // rng: rand::rng(),
        }
    }

    fn run(&mut self) {
        'loop_label: loop {
            // TODO: use select_biased! to prioritize simulation controller commands
            select! {
                recv(self.packet_recv) -> packet_res => {
                    match packet_res {
                        Err(_err) => {
                             match &self.state {
                                 State::Working => {
                                     panic!("There is no connected sender to the drone's receiver channel and no DroneCommand::Crash has been received")
                                 },
                                 State::Crashing => {
                                     break 'loop_label
                                 },
                             }
                        }
                        Ok(packet) => {
                            self.process_packet(packet);
                        }
                    }
                },
                recv(self.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        match command
                        {
                            DroneCommand::AddSender(node_id, sender) => {
                                self.add_channel(node_id, sender);
                            },
                            DroneCommand::SetPacketDropRate(pdr) => {
                                self.set_pdr(pdr);
                            },
                            DroneCommand::Crash => {
                                self.state = State::Crashing;
                            },
                            DroneCommand::RemoveSender(node_id_to_be_removed) => {
                                match self.packet_send.remove(&node_id_to_be_removed) {
                                    Some(_removed_channel) => {
                                        log::info!("Channel to {node_id_to_be_removed} removed successfully");
                                    },
                                    None => {
                                        panic!("Cannot remove channel to {node_id_to_be_removed}: it does not exist")
                                    },
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

impl MyDrone {
    fn process_packet(&self, mut packet: Packet) {
        // step 1 (merged with step 3)
        let current_index = packet.routing_header.hop_index;
        if current_index >= packet.routing_header.hops.len() {
            let nack = Self::create_nack_packet(&packet, NackType::UnexpectedRecipient(self.id));
            self.send_packet_to_neighbor(nack);
            return;
        }

        // TODO: I guess that this sends NackType::DestinationIsDrone when receiving flood requests, which is wrong
        if current_index == packet.routing_header.hops.len() - 1 {
            let nack = Self::create_nack_packet(&packet, NackType::DestinationIsDrone);
            self.send_packet_to_neighbor(nack);
            return;
        }

        let current_hop = packet.routing_header.hops.get(current_index).unwrap();
        if packet.routing_header.hops[*current_hop as usize] != self.id {
            let nack = Self::create_nack_packet(&packet, NackType::UnexpectedRecipient(self.id));
            self.send_packet_to_neighbor(nack);
            return;
        }

        // step 2
        packet.routing_header.hop_index += 1;

        self.send_packet_to_neighbor(packet);
    }

    fn create_nack_packet(original_packet: &Packet, nack_type: NackType) -> Packet {
        let fragment_index = match &original_packet.pack_type {
            PacketType::MsgFragment(frag) => frag.fragment_index,
            // if the packet is not a fragment it is considered as a whole so frag index is 0
            _ => 0,
        };
        let inner_nack = Nack {
            fragment_index,
            nack_type,
        };

        let hop_index = original_packet.routing_header.hop_index;
        // TODO: check correctness
        let reverse_path = original_packet
            .routing_header
            .hops
            .split_at(hop_index + 1)
            .0
            .iter()
            .rev()
            .copied()
            .collect::<Vec<NodeId>>();

        Packet {
            pack_type: PacketType::Nack(inner_nack),
            routing_header: SourceRoutingHeader {
                hop_index: 1,
                hops: reverse_path,
            },
            session_id: original_packet.session_id,
        }
    }

    /// panics if `packet.routing_header.hop_index` is not a key of `self.packet_send`
    fn send_packet_to_neighbor(&self, packet: Packet) {
        let next_hop = packet.routing_header.hops.get(packet.routing_header.hop_index).unwrap();
        let channel = self.packet_send.get(next_hop).unwrap();

        match &packet.pack_type {
            PacketType::MsgFragment(_fragment) => {
                if !self.packet_send.contains_key(next_hop) {
                    let nack = Self::create_nack_packet(&packet, NackType::ErrorInRouting(*next_hop));
                    self.send_packet_to_neighbor(nack);
                    return;
                }

                // TODO: fix rand::rng();
                let mut rng = rand::rng();
                let random_number: f32 = rng.random_range(0.0..=1.0);

                if random_number < self.pdr {
                    let nack = Self::create_nack_packet(&packet, NackType::Dropped);
                    self.send_packet_to_neighbor(nack);
                } else {
                    self.send_packet_and_notify_simulation_controller(channel, packet);
                }
            }
            PacketType::Nack(_) | PacketType::Ack(_) | PacketType::FloodResponse(_) => {
                if !self.packet_send.contains_key(next_hop) {
                    let event = DroneEvent::ControllerShortcut(packet);
                    self.send_event_to_simulation_controller(&event);
                    return;
                }
                self.send_packet_and_notify_simulation_controller(channel, packet);
            }
            PacketType::FloodRequest(flood_request) => {
                if !self.packet_send.contains_key(next_hop) {
                    // flood request does not send NACKs as the current protocol does not specify it
                    return;
                }
                // TODO: handle flood request logic

                let received_from = match flood_request.path_trace.last() {
                    Some((node_id, _node_type)) => node_id,
                    None => panic!("flood request has no path trace"), // TODO: decide if it is appropriate to panic
                };
                let flood_id = flood_request.flood_id;
                let mut new_path_trace = flood_request.path_trace.clone();
                new_path_trace.push((self.id, NodeType::Drone));

                let drone_has_no_other_neighbors = self
                    .packet_send
                    .iter()
                    .filter(|(node_id, _channel)| **node_id != *received_from)
                    .count() == 0;

                if self.known_flood_ids.contains(&flood_id) || drone_has_no_other_neighbors {
                    let flood_response = PacketType::FloodResponse (
                        FloodResponse {
                            flood_id,
                            path_trace: new_path_trace,
                        }
                    );
                    let new_hops: Vec<NodeId> = packet.routing_header.hops.iter().rev().copied().collect();
                    let flood_response_packet = Packet {
                        pack_type: flood_response,
                        routing_header: SourceRoutingHeader {
                            hop_index: 1,
                            hops: new_hops,
                        },
                        session_id: packet.session_id,
                    };
                    self.send_packet_to_neighbor(flood_response_packet);
                } else {
                    // TODO: check this else branch again
                    let flood_request = FloodRequest {
                        flood_id,
                        initiator_id: flood_request.initiator_id,
                        path_trace: new_path_trace,
                    };
                    let packet_type = PacketType::FloodRequest(flood_request);
                    let previous_routing_header = packet.routing_header.clone();

                    let next_hops_to_forward_packet_to: Vec<NodeId> = self
                        .packet_send
                        .iter()
                        .filter(|(node_id, _channel)| **node_id != *received_from)
                        .map(|(node_id, _channel)| node_id)
                        .copied()
                        .collect();

                    for next_hop in &next_hops_to_forward_packet_to {
                        let mut routing_header = previous_routing_header.clone();
                        routing_header.hops.push(*next_hop);
                        let packet = Packet {
                            pack_type: packet_type.clone(),
                            routing_header,
                            session_id: packet.session_id,
                        };
                        self.send_packet_to_neighbor(packet);
                    }
                }
            }
        }
    }

    /// panics if `channel.send()` fails
    fn send_packet_and_notify_simulation_controller(&self, channel: &Sender<Packet>, packet: Packet) {
        match channel.send(packet.clone()) {
            Ok(()) => {
                log::info!("Packet {:?} successfully sent into channel {channel:?}", &packet);
                let drone_event = DroneEvent::PacketSent(packet);
                self.send_event_to_simulation_controller(&drone_event);
            }
            Err(error) => {
                panic!("Cannot send packet {:?} into channel {channel:?}. Error: {error:?}", &packet);
            }
        }
    }

    /// panics if `self.controller_send.send()` fails
    fn send_event_to_simulation_controller(&self, event: &DroneEvent) {
        match self.controller_send.send(event.clone()) {
            Ok(()) => {
                log::info!("Event {:?} successfully sent to simulation controller", &event);
            }
            Err(error) => {
                panic!("Cannot send event {:?} to simulation controller. Error: {error:?}", &event);
            }
        }
    }

    /// Sets `self.pdr` to the given `pdr` value.
    /// Panics if `pdr` is not in the valid range
    fn set_pdr(&mut self, pdr: f32) {
        assert!((0f32..=1f32).contains(&pdr), "Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)");
        self.pdr = pdr;
        log::info!("pdr updated to {pdr}");
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        match self.packet_send.insert(id, sender) {
            Some(_previous_sender) => {
                log::info!("Sender to node {id} updated");
            },
            None => {
                log::info!("Sender to node {id} inserted");
            },
        }
    }
}
