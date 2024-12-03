use core::panic;
use crossbeam_channel::{select, Receiver, Sender};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Nack, NackType, Packet, PacketType};

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
        }
    }

    fn run(&mut self) {
        'loop_label: loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    match packet_res {
                        Err(err) => {
                            match err {
                                _recv_error => {
                                    match &self.state {
                                        State::Working => {
                                            panic!("There is no connected sender to the drone's receiver channel and no DroneCommand::Crash has been received")
                                        },
                                        State::Crashing => {
                                            break 'loop_label
                                        },
                                    }
                                }
                            }
                        }
                        Ok(packet) => {
                            match &packet.pack_type {
                            PacketType::Nack(_) | PacketType::Ack(_) => {
                                match self.forward_packet(packet.clone()) {
                                    Ok(()) => {
                                        let sent_packet = DroneEvent::PacketSent(packet.clone());
                                        match self.controller_send.send(sent_packet) {
                                            Ok(()) => {
                                                // packet successfully sent to simulation controller
                                            },
                                            Err(send_error) => {
                                                // error while sending packet to simulation controller
                                                panic!("{send_error:?}")
                                            }
                                        }
                                    },
                                    Err(error) => {
                                        let error_packet = create_error_packet(&packet, error);
                                        let _ = self.forward_packet(error_packet);
                                    }
                                }
                            },
                                /*
                            // PacketType::Ack(_ack) => {
                            //     match self.forward_packet(packet.clone()) {
                            //         Ok(()) => {},
                            //         Err(error) => {
                            //             let error_packet = create_error_packet(&packet, error);
                            //             let _ = self.forward_packet(error_packet);
                            //         }
                            //     }
                            // },
                           */
                            PacketType::MsgFragment(fragment) => {
                                let fragment_index = fragment.fragment_index;
                                let mut rng = rand::rng();
                                let random_number: f32 = rng.random_range(0.0..=1.0);
                                if random_number <= self.pdr {
                                    let error_packet = create_error_packet(&packet,Nack{fragment_index,nack_type: NackType::Dropped}) ;
                                    let _ = self.forward_packet(error_packet);
                                } else {
                                    match self.forward_packet(packet.clone()) {
                                        Ok(()) => {
                                            let sent_packet = DroneEvent::PacketSent(packet.clone());
                                            match self.controller_send.send(sent_packet) {
                                                Ok(()) => {
                                                    // packet successfully sent to simulation controller
                                                },
                                                Err(send_error) => {
                                                    // error while sending packet to simulation controller
                                                    panic!("{send_error:?}")
                                                }
                                            }
                                        },
                                        Err(error) => {
                                            let error_packet = create_error_packet(&packet, error);
                                            let _ = self.forward_packet(error_packet);
                                        }
                                    }
                                }
                            },
                            PacketType::FloodRequest(_) => {

                                },
                            PacketType::FloodResponse(_) => {
                                    unimplemented!()
                                },
                            }
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
                                self.state = State::Crashing,
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

fn create_error_packet(original_packet: &Packet, nack: Nack) -> Packet {
    let hop_index = original_packet.routing_header.hop_index;
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
        pack_type: PacketType::Nack(nack),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: reverse_path,
        },
        session_id: original_packet.session_id,
    }
}

impl MyDrone {
    // TODO: ask what to do if a nack/ack packet contains invalid routing -> ???
    // Note: by protocol constraint, a drone cannot check hop_index field.
    // Therefore, you have to rely on other types of checks to verify whether a drone
    // is the destination of a packet and, if so, signal the logical error
    fn forward_packet(&self, mut packet: Packet) -> Result<(), Nack> {
        packet.routing_header.hop_index += 1;
        let hop_index = packet.routing_header.hop_index;
        let next_hop = packet.routing_header.hops.get(hop_index);
        // cloned below because otherwise we have a partial borrow error in the next match
        let fragment_index = match packet.pack_type.clone() {
            PacketType::MsgFragment(frag) => frag.fragment_index,
            // if the packet is not a fragment it is considered as a whole so frag index is 0
            _ => 0,
        };
        match next_hop {
            Some(next_node) => match self.packet_send.get(next_node) {
                Some(next_node_channel) => {
                    let _ = next_node_channel.send(packet);
                    Ok(())
                }
                None => Err(Nack {
                    fragment_index,
                    nack_type: NackType::ErrorInRouting(*next_node),
                }),
            },
            None => {
                // if the match expression returns None, it means that the current drone
                // is the designed destination
                Err(Nack {
                    fragment_index,
                    nack_type: NackType::DestinationIsDrone,
                })
            }
        }
    }

    fn set_pdr(&mut self, pdr: f32) {
        assert!((0f32..=1f32).contains(&pdr), "Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)");
        self.pdr = pdr;
    }

    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}
