use core::panic;
use crossbeam_channel::{select, Receiver, Sender};
use log::warn;
use rand::Rng;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::usize;
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
    known_flood_ids: RefCell<HashSet<(u64, NodeId)>>,
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
        assert!(
            !packet_send.contains_key(&id),
            "neighbor with id {id} which is the same as drone"
        );
        assert!((0.0..=1.0).contains(&pdr), "pdr out of bounds");
        Self {
            id,
            controller_send,
            controller_recv,
            packet_recv,
            pdr,
            packet_send,
            // TODO: I don't understand why we need to use RefCell here???
            known_flood_ids: RefCell::new(HashSet::new()),
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
    /// Sets `self.pdr` to the given `pdr` value.
    /// Panics if `pdr` is not in the valid range
    fn set_pdr(&mut self, pdr: f32) {
        assert!(
            (0f32..=1f32).contains(&pdr),
            "Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)"
        );
        self.pdr = pdr;
        log::info!("pdr updated to {pdr}");
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        match self.packet_send.insert(id, sender) {
            Some(_previous_sender) => {
                log::info!("Sender to node {id} updated");
            }
            None => {
                log::info!("Sender to node {id} inserted");
            }
        }
    }
}

// packet processing section
impl MyDrone {
    pub fn process_packet(&self, mut packet: Packet) {
        match packet.pack_type {
            // flood requests do not care about source routing header so there is another logic for
            // them
            PacketType::FloodRequest(_) => self.process_flood_request(packet),
            _ => self.process_not_flood_request(packet),
        }
    }

    fn process_not_flood_request(&self, mut packet: Packet) {
        let current_index = packet.routing_header.hop_index;

        if packet.routing_header.is_empty() {
            panic!("empty routing header for packet {packet}")
        }

        if matches!(packet.pack_type, PacketType::FloodRequest(_)) {
            panic!("not expecting a packet of type flood request")
        }
        // TODO: decide if this should be a panic, when decided document it in readme because it's
        // not a behavior well defined by the protocol
        if current_index >= packet.routing_header.hops.len() {
            panic!(
                "hop_index out of bounds: index {current_index} for hops {:?}",
                packet.routing_header.hops
            );
        }

        let current_hop = packet.routing_header.hops.get(current_index).unwrap();
        if *current_hop != self.id {
            self.make_and_send_nack(
                &packet,
                current_index,
                NackType::UnexpectedRecipient(self.id),
            );
            return;
        }

        if packet.routing_header.is_last_hop() {
            self.make_and_send_nack(&packet, current_index, NackType::DestinationIsDrone);
            return;
        }

        // important because when using send_packet in the following code we want to pass it a
        // packet with the hop_idx alreay pointing to the destination
        packet.routing_header.hop_index += 1;

        self.send_packet(packet);
    }

    fn process_flood_request(&self, packet: Packet) {
        let flood_request = match packet.pack_type {
            PacketType::FloodRequest(f) => f,
            _ => panic!("expecting a packet of type flood request"),
        };

        let received_from = match flood_request.path_trace.last() {
            Some((node_id, _node_type)) => node_id,
            None => panic!("flood request has no path trace"), // TODO: decide if it is appropriate to panic
        };

        let flood_id = flood_request.flood_id;
        let initiator_id = flood_request.initiator_id;
        let mut new_path_trace = flood_request.path_trace.clone();
        new_path_trace.push((self.id, NodeType::Drone));

        let drone_has_no_other_neighbors = self
            .packet_send
            .iter()
            .filter(|(node_id, _channel)| **node_id != *received_from)
            .count()
            == 0;

        if self
            .known_flood_ids
            .borrow()
            .contains(&(flood_id, initiator_id))
            || drone_has_no_other_neighbors
        {
            let flood_response = PacketType::FloodResponse(FloodResponse {
                flood_id,
                path_trace: new_path_trace.clone(),
            });
            // there is no hops vec to reverse in sourcerouting header because
            // floodrequest ignores it, path trace is used instead

            let hops: Vec<NodeId> = new_path_trace
                .iter()
                .map(|(id, _)| id)
                .rev()
                .copied()
                .collect();

            let flood_response_packet = Packet {
                pack_type: flood_response,
                routing_header: SourceRoutingHeader { hop_index: 1, hops },
                session_id: packet.session_id,
            };
            self.send_packet(flood_response_packet);
        } else {
            self.known_flood_ids
                .borrow_mut()
                .insert((flood_id, initiator_id));

            let flood_request = FloodRequest {
                flood_id,
                initiator_id: flood_request.initiator_id,
                path_trace: new_path_trace,
            };
            let packet_type = PacketType::FloodRequest(flood_request);

            let next_hops_to_forward_packet_to: Vec<NodeId> = self
                .packet_send
                .iter()
                .filter(|(node_id, _channel)| **node_id != *received_from)
                .map(|(node_id, _channel)| node_id)
                .copied()
                .collect();

            for next_hop in &next_hops_to_forward_packet_to {
                // even though the routing header is not used by the flooding protocol we need it
                // here because:
                // - send_packet uses it to know where to send the packet
                // - when logging we always show the Packet sourceroutingHeader, and this gives use
                // more information
                // TODO: behavior not defined in protocol so it's worth to document it in the
                // readme if we keep it
                let routing_header = SourceRoutingHeader {
                    hop_index: 1,
                    hops: vec![self.id, *next_hop],
                };
                let packet = Packet {
                    pack_type: packet_type.clone(),
                    routing_header,
                    session_id: packet.session_id,
                };

                self.send_packet(packet);
            }
        }
    }
}

// packet sending section
impl MyDrone {
    /// takes a packet whose routing header hop index already points to the intended destination
    /// sends that packet through the `channel` corresponding to the current hop index, panics if there is a SendError
    fn send_packet(&self, packet: Packet) {
        // TODO: we could use some checks at least that hop_idx and hop_idx+1 are contained in the
        // hops

        // use hop_idx to get id of destination:
        let dest = packet
            .routing_header
            .current_hop()
            .expect("next hop not found");

        if let Some(channel) = self.packet_send.get(&dest) {
            // packet drop logic
            if matches!(packet.pack_type, PacketType::MsgFragment(_)) {
                // TODO: fix rand::rng();
                let mut rng = rand::rng();
                let random_number: f32 = rng.random_range(0.0..=1.0);

                if random_number < self.pdr {
                    self.make_and_send_nack(
                        &packet,
                        packet.routing_header.hop_index - 1,
                        NackType::Dropped,
                    );
                    return;
                }
            }

            match channel.send(packet.clone()) {
                Ok(()) => {
                    let drone_event = DroneEvent::PacketSent(packet.clone());
                    self.send_event(&drone_event);
                    log::info!("Sent to channel of Drone#{} Packet {}", dest, &packet,);
                }
                Err(error) => {
                    panic!(
                        "Cannot send packet {} into channel {channel:?}. Error: {error:?}",
                        &packet
                    );
                }
            }
        } else {
            match packet.pack_type.clone() {
                PacketType::MsgFragment(_) | PacketType::FloodRequest(_) => {
                    let idx = packet.routing_header.previous_hop().unwrap();
                    self.make_and_send_nack(&packet, idx as usize, NackType::ErrorInRouting(dest));
                }
                _ => {
                    let event = DroneEvent::ControllerShortcut(packet);
                    self.send_event(&event);
                }
            }
        };
    }
    /// sends an event to the simulation controller
    /// panics if `self.controller_send.send()` fails
    pub fn send_event(&self, event: &DroneEvent) {
        match self.controller_send.send(event.clone()) {
            Ok(()) => {
                let (event_type, packet) = match event {
                    DroneEvent::PacketSent(packet) => ("PacketSent", packet),
                    DroneEvent::PacketDropped(packet) => ("PacketDropped", packet),
                    DroneEvent::ControllerShortcut(packet) => ("ControllerShortcut", packet),
                };
                log::debug!(
                    "Sent DroneEvent::{} to simulation controller, about packet{}",
                    event_type,
                    packet,
                );
            }
            Err(error) => {
                panic!(
                    "Cannot send event {:?} to simulation controller. Error: {error:?}",
                    &event
                );
            }
        }
    }

    /// all nacks that are generated by this drone pass through here:
    /// creates and sends a nack with the given NackType, containing the `original_packet` and reversing the
    /// route so that it goes from `original_recipient_idx` to the node that sent `original_packet`
    /// (the one at index 0)
    fn make_and_send_nack(
        &self,
        original_packet: &Packet,
        original_recipient_idx: usize,
        nack_type: NackType,
    ) {
        // TODO: add checks for original_recipient_idx or alternatively do it like with packet_send
        // and use hop_index to find out the original_recipient
        let fragment_index = match &original_packet.pack_type {
            PacketType::MsgFragment(frag) => frag.fragment_index,
            // if the packet is not a fragment it is considered as a whole so frag index is 0
            _ => 0,
        };

        let nack = Nack {
            fragment_index,
            nack_type,
        };

        let mut new_hops = original_packet.routing_header.hops[0..=original_recipient_idx].to_vec();
        new_hops.reverse();

        let new_header = SourceRoutingHeader {
            hop_index: 1,
            hops: new_hops,
        };
        let packet = Packet {
            pack_type: PacketType::Nack(nack),
            routing_header: new_header,
            session_id: original_packet.session_id,
        };

        if matches!(nack_type, NackType::Dropped) {
            self.send_event(&DroneEvent::PacketDropped(original_packet.clone()));

            // TODO: another small detail not too clear in the protocol: here we send the
            // original_packet which had already his hop_index increased so its pointing to
            // where we would have sent him if everything went ok, other option is having
            // hop_index-1 as it was when packet arrived.
            // The fact that we choose one of the two IMO should be documented in the
            // readme, because it could be unexpected when viewing the list of dropped packets in
            // the simulation controller
        }
        self.send_packet(packet);
    }
}
