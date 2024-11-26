use crossbeam_channel::{select, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use rand::Rng;
use wg_2024::controller::{DroneCommand, NodeEvent};
use wg_2024::drone::{Drone, DroneOptions};
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Nack, Packet, PacketType};

pub struct MyDrone {
    id: NodeId,
    sim_contr_send: Sender<NodeEvent>,
    sim_contr_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
    known_flood_ids: HashSet<u64>, // TODO: use this field
}

impl Drone for MyDrone {
    fn new(options: DroneOptions) -> Self {
        Self {
            id: options.id,
            sim_contr_send: options.controller_send,
            sim_contr_recv: options.controller_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr,
            packet_send: HashMap::new(),
            known_flood_ids: HashSet::new(),
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match &packet.pack_type {
                            PacketType::Nack(_nack) | PacketType::Ack(_ack) => {
                                match self.forward_packet(packet.clone()) {
                                    Ok(()) => {
                                        let sent_packet = NodeEvent::PacketSent(packet.clone())
                                        match self.sim_contr_send.send(sent_packet) {
                                            Ok(()) => {
                                                // packet successfully sent to simulation controller
                                            },
                                            Err(send_error) => {
                                                // error while sending packet to simulation controller
                                                panic!("{:?}", send_error)
                                            }
                                        }
                                    },
                                    Err(error) => {
                                        let error_packet = create_error_packet(&packet, error);
                                        let _ = self.forward_packet(error_packet);
                                    }
                                }
                            },
                            // PacketType::Ack(_ack) => {
                            //     match self.forward_packet(packet.clone()) {
                            //         Ok(()) => {},
                            //         Err(error) => {
                            //             let error_packet = create_error_packet(&packet, error);
                            //             let _ = self.forward_packet(error_packet);
                            //         }
                            //     }
                            // },
                            PacketType::MsgFragment(fragment) => {
                                let fragment_index = fragment.fragment_index;
                                let mut rng = rand::rng();
                                let random_number: f32 = rng.random_range(0.0..=1.0);
                                if random_number <= self.pdr {
                                    let error_packet = create_error_packet(&packet, Nack::Dropped(fragment_index));
                                    let _ = self.forward_packet(error_packet);
                                } else {
                                    match self.forward_packet(packet.clone()) {
                                        Ok(()) => {
                                            let sent_packet = NodeEvent::PacketSent(packet.clone())
                                            match self.sim_contr_send.send(sent_packet) {
                                                Ok(()) => {
                                                    // packet successfully sent to simulation controller
                                                },
                                                Err(send_error) => {
                                                    // error while sending packet to simulation controller
                                                    panic!("{:?}", send_error)
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
                            PacketType::FloodRequest(_) => unimplemented!(),
                            PacketType::FloodResponse(_) => unimplemented!(),
                        }
                    }
                },
                recv(self.sim_contr_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        match command{
                            DroneCommand::AddSender(_, sender) => todo!(),
                            DroneCommand::SetPacketDropRate(pdr) => todo!(),
                            DroneCommand::Crash => todo!(),
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
            hops: reverse_path
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
        match next_hop {
            Some(next_node) => {
                match self.packet_send.get(next_node) {
                    Some(next_node_channel) => {
                        let _ = next_node_channel.send(packet);
                        Ok(())
                    },
                    None => {
                        Err(Nack::ErrorInRouting(*next_node))
                    }
                }
            },
            None => {
                // if the match expression returns None, it means that the current drone
                // is the designed destination
                Err(Nack::DestinationIsDrone)
            }
        }
    }


    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }
}