use core::panic;
use crossbeam_channel::{select_biased, Receiver, Sender};
use rand::rngs::ThreadRng;
use rand::Rng;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, Nack, NackType, NodeType, Packet, PacketType};

#[derive(Clone, Copy, Debug)]
enum State {
    Working,
    Crashing,
}

#[derive(Debug)]
pub struct MyDrone {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    known_flood_ids: HashSet<(u64, NodeId)>,
    state: State,
}

// Thread-local storage for `ThreadRng`
thread_local! {
    static RNG: RefCell<ThreadRng> = RefCell::new(rand::rng());
}

fn generate_random_value_in_range(range: RangeInclusive<f32>) -> f32 {
    RNG.with(|rng| rng.borrow_mut().random_range(range)) // Use the thread-local RNG
}

impl Drone for MyDrone {
    /// creates the drone with the given options, logs that it has been created and returns the
    /// drone
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        let mut result = Self {
            id,
            controller_send,
            controller_recv,
            packet_recv,
            pdr: 0f32,
            packet_send: HashMap::new(),
            known_flood_ids: HashSet::new(),
            state: State::Working,
        };
        result.set_pdr(pdr);
        for (node_id, channel) in packet_send {
            result.add_channel(node_id, channel);
        }
        log::info!("\"null-pointer-drone\" drone created: {:?}", result);
        result
    }

    /// runs the drone loop, listening for events and packets, exits succesfully if sent a
    /// `DroneCommand::Crash` after all references of the sender for its own receiver have been
    /// dropped
    ///
    /// # Panics
    /// ## The Sender<DroneCommand> end of the simulation controller channel unexpectedly got dropped
    ///
    /// ## There is no connected sender to the drone's receiver channel and no DroneCommand::Crash has been received
    ///
    /// ## Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)
    ///
    /// ## Cannot add a channel with the same NodeId of this drone (which is {})
    /// You cannot connect the drone to itself or another drone with the same id
    ///
    /// ## Cannot remove channel to {node_id}: it does not exist
    ///
    /// ## empty routing header for packet {packet}
    ///
    /// ## hop_index out of bounds: index {current_index} for hops {:?}
    ///
    /// ## received packet with hop_index 0, which should be impossible
    /// Refers to packets that are not a flood request, maybe this packet was supposed to be a **flood request**
    ///
    /// ## flood request has no path trace
    ///
    /// ## Cannot send packet {&packet} into channel {channel:?}. Error: {error:?}
    /// when the crossbeam channel send returns an error
    ///
    /// ## Cannot send event {&event} to simulation controller. Error: {error:?}"
    /// same thing as above but for event
    fn run(&mut self) {
        'loop_label: loop {
            /*
               From https://shadow.github.io/docs/rust/crossbeam/channel/macro.select_biased.html
               "If multiple operations are ready at the same time, the operation nearest to the front of the list is always selected"
               So recv(self.controller_recv) needs to come before recv(self.packet_recv)

               WARNING: select_biased! seems to select a channel even if one end of it is dropped
                        for instance, if I create a drone and drop the Sender<DroneCommand> channel and I keep using the drone,
                        then the drone will always select the recv(self.controller_recv) arm, since it receives an Err().
                        This means that the drone stops working properly in the case of bad channels management.
                        Interestingly, this didn't occur with the select! macro
            */
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    log::info!("Received controller command: {command_res:?}");
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
                                self.set_state(State::Crashing);
                            },
                            DroneCommand::RemoveSender(node_id_to_be_removed) => {
                                self.remove_channel(node_id_to_be_removed);
                            },
                        }
                    } else {
                        panic!("The Sender<DroneCommand> end of the simulation controller channel unexpectedly got dropped");
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    log::info!("Received packet: {packet_res:?}");
                    match packet_res {
                        Err(_err) => {
                             match &self.state {
                                 State::Working => {
                                     panic!("There is no connected sender to the drone's packet receiver channel and no DroneCommand::Crash has been received")
                                 },
                                 State::Crashing => {
                                    log::info!("Drone is finally crashing, no more packets will be processed");
                                    break 'loop_label
                                 },
                             }
                        }
                        Ok(packet) => {
                            log::info!("Processing packet {packet}");
                            self.process_packet(packet);
                        }
                    }
                },
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
        log::info!("pdr set to {pdr}");
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        assert_ne!(
            id, self.id,
            "Cannot add a channel with the same NodeId of this drone (which is {})",
            self.id
        );

        match self.packet_send.insert(id, sender) {
            Some(_previous_sender) => {
                log::info!("Sender channel to node {id} updated");
            }
            None => {
                log::info!("Sender channel to node {id} inserted");
            }
        }
    }

    fn remove_channel(&mut self, node_id: NodeId) {
        match self.packet_send.remove(&node_id) {
            Some(_removed_channel) => {
                log::info!("Channel to {node_id} removed successfully");
            }
            None => {
                panic!("Cannot remove channel to {node_id}: it does not exist")
            }
        }
    }

    fn set_state(&mut self, state: State) {
        self.state = state;
        log::info!("state set to {state:?}");
    }
}

// packet processing section
impl MyDrone {
    pub fn process_packet(&mut self, packet: Packet) {
        match packet.pack_type {
            PacketType::FloodRequest(flood_request) => {
                self.process_flood_request(flood_request, packet.session_id);
            }
            _ => self.process_not_flood_request(packet),
        }
    }

    fn process_not_flood_request(&mut self, mut packet: Packet) {
        // you never know what could happen
        debug_assert!(!matches!(packet.pack_type, PacketType::FloodRequest(_)));

        let current_index = packet.routing_header.hop_index;

        assert!(
            !packet.routing_header.is_empty(),
            "empty routing header for packet {packet}"
        );

        assert!(
            current_index < packet.routing_header.hops.len(),
            "hop_index out of bounds: index {current_index} for hops {:?}",
            packet.routing_header.hops
        );

        assert!(
            current_index != 0,
            "received packet with hop_index 0, which should be impossible"
        );

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
            log::warn!(
                "Drone is the destination of the packet, sending back {:?}",
                NackType::DestinationIsDrone
            );
            self.make_and_send_nack(&packet, current_index, NackType::DestinationIsDrone);
            return;
        }

        // important because when using send_packet in the following code we want to pass it a
        // packet with the hop_idx alreay pointing to the destination
        packet.routing_header.hop_index += 1;

        self.send_packet(packet);
    }

    fn process_flood_request(&mut self, flood_request: FloodRequest, session_id: u64) {
        let FloodRequest {
            flood_id,
            initiator_id,
            mut path_trace,
        } = flood_request;

        let Some((received_from, _)) = path_trace.last().copied() else {
            panic!("flood request has no path trace")
        };

        path_trace.push((self.id, NodeType::Drone));

        let neighbors_minus_sender: Vec<NodeId> = self
            .packet_send
            .iter()
            .filter(|(node_id, _channel)| **node_id != received_from)
            .map(|(node_id, _channel)| node_id)
            .copied()
            .collect();

        let drone_has_no_other_neighbors = neighbors_minus_sender.is_empty();

        if self.known_flood_ids.contains(&(flood_id, initiator_id)) || drone_has_no_other_neighbors
        {
            if drone_has_no_other_neighbors {
                log::debug!("Drone has no other neighbors except for the sender, generating flood response...");
            } else {
                log::debug!("tuple (flood_id:{},initiator_id:{}) already seen, generating flood response...",flood_id,initiator_id);
            }

            let flood_response = PacketType::FloodResponse(FloodResponse {
                flood_id,
                path_trace: path_trace.clone(),
            });
            // there is no hops vec to reverse in sourcerouting header because
            // floodrequest ignores it, path trace is used instead

            let hops: Vec<NodeId> = path_trace.iter().map(|(id, _)| id).rev().copied().collect();

            let flood_response_packet = Packet {
                pack_type: flood_response,
                routing_header: SourceRoutingHeader { hop_index: 1, hops },
                session_id,
            };
            self.send_packet(flood_response_packet);
        } else {
            log::debug!(
                "found neighbors(except sender): {:?}, forwarding flood request...",
                neighbors_minus_sender
            );
            self.known_flood_ids.insert((flood_id, initiator_id));

            let flood_request = FloodRequest {
                flood_id,
                initiator_id: flood_request.initiator_id,
                path_trace,
            };
            let packet_type = PacketType::FloodRequest(flood_request);

            for next_hop in &neighbors_minus_sender {
                // even though the routing header is not used by the flooding protocol we need it
                // here because:
                // - send_packet uses it to know where to send the packet
                // - when logging we always show the Packet sourceroutingHeader, and this gives use
                // more information
                let routing_header = SourceRoutingHeader {
                    hop_index: 1,
                    hops: vec![self.id, *next_hop],
                };
                let packet = Packet {
                    pack_type: packet_type.clone(),
                    routing_header,
                    session_id,
                };

                self.send_packet(packet);
            }
        }
    }
}

// packet sending section
impl MyDrone {
    /// takes a packet whose routing header hop index already points to the intended destination
    /// sends that packet through the `channel` corresponding to the current hop index, panics if there is a `SendError`
    fn send_packet(&self, mut packet: Packet) {
        // use hop_idx to get id of destination:
        let dest = packet
            .routing_header
            .current_hop()
            .expect("next hop not found: the internal function packet_send was passed a packet with no destination(its hop_index, which was incremented by 1 to point to dest, was out of bounds for hops)");

        if let Some(channel) = self.packet_send.get(&dest) {
            // packet drop logic
            if matches!(packet.pack_type, PacketType::MsgFragment(_))
                && self.roll_a_dice_and_decide_maybe_drop_packet()
            {
                log::info!("Dropping packet due to drone's pdr");
                packet.routing_header.hop_index -= 1;
                self.make_and_send_nack(
                    &packet,
                    packet.routing_header.hop_index,
                    NackType::Dropped,
                );
                return;
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
            match &packet.pack_type {
                PacketType::MsgFragment(_) => {
                    log::info!(
                        "Next hop of header is not a neighbor of drone, creating {:?}",
                        NackType::ErrorInRouting(dest)
                    );
                    let idx = packet.routing_header.previous_hop().unwrap();
                    self.make_and_send_nack(&packet, idx as usize, NackType::ErrorInRouting(dest));
                }
                PacketType::FloodRequest(_) => {
                    unreachable!("Flood request algorithm should never try to send a flood request to a node not in the list of neighbors, as it gets all the neighbors from that list");
                }
                _ => {
                    log::info!("Sending packet {packet} to simulation controller to shortcut it");
                    let event = DroneEvent::ControllerShortcut(packet);
                    self.send_event(&event);
                }
            }
        };
    }

    /// This method handles the logic of the packet dropping.
    /// It decides to drop a packet or not and, if it does, it creates and sends the related
    /// NACK packet
    fn roll_a_dice_and_decide_maybe_drop_packet(&self) -> bool {
        let random_number: f32 = generate_random_value_in_range(0.0..=1.0);
        random_number < self.pdr
    }

    /// sends an event to the simulation controller
    /// # Panics
    /// Panics if `self.controller_send.send()` fails
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
    /// creates and sends a nack with the given `NackType`, containing the `original_packet` and reversing the
    /// route so that it goes from `original_recipient_idx` to the node that sent `original_packet`
    /// (the one at index 0)
    fn make_and_send_nack(
        &self,
        original_packet: &Packet,
        original_recipient_idx: usize,
        nack_type: NackType,
    ) {
        //expected to be unreachable given the logic of where make and send nack is done
        assert!(
            original_packet
                .routing_header
                .hops
                .get(original_recipient_idx)
                .is_some(),
            "original recipient index out of bounds, this should be unreachable"
        );

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

            // another small detail not too clear in the protocol: here we send the
            // original_packet which had already his hop_index increased so its pointing to
            // where we would have sent him if everything went ok, other option is having
            // hop_index-1 as it was when packet arrived.
        }
        self.send_packet(packet);
    }
}
