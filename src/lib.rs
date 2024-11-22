use crossbeam_channel::{select, Receiver, Sender};
use std::collections::HashMap;
use std::time::{Instant};
use rand::Rng;
use wg_2024::controller::Command;
use wg_2024::drone::Drone;
use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{Nack, NackType, Packet, PacketType};

// TODO: add missing field 'known_flood_ids: HashSet<u64>' to keep track of previous flood requests
pub struct MyDrone {
    id: NodeId,
    sim_contr_send: Sender<Command>,
    sim_contr_recv: Receiver<Command>,
    packet_recv: Receiver<Packet>,
    pdr: f32,
    packet_send: HashMap<NodeId, Sender<Packet>>,
}

impl Drone for MyDrone {
    fn new(options: wg_2024::drone::DroneOptions) -> Self {
        Self {
            id: options.id,
            sim_contr_send: options.sim_contr_send,
            sim_contr_recv: options.sim_contr_recv,
            packet_recv: options.packet_recv,
            pdr: options.pdr * 100.0,
            packet_send: HashMap::new(),
        }
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        match &packet.pack_type {
                            PacketType::Nack(nack) => {
                                let fragment_index = nack.fragment_index;
                                match self.forward_packet(packet.clone()) {
                                    Ok(()) => {},
                                    Err(error) => {
                                        let error_packet = create_error_packet(&packet, error, fragment_index);
                                        let _ = self.forward_packet(error_packet);
                                    }
                                }
                            },
                            PacketType::Ack(ack) => {
                                let fragment_index = ack.fragment_index;
                                match self.forward_packet(packet.clone()) {
                                    Ok(()) => {},
                                    Err(error) => {
                                        let error_packet = create_error_packet(&packet, error, fragment_index);
                                        let _ = self.forward_packet(error_packet);
                                    }
                                }
                            },
                            PacketType::MsgFragment(fragment) => {
                                let fragment_index = fragment.fragment_index;
                                let mut rng = rand::thread_rng();
                                let random_number: f32 = rng.gen_range(0.0..=1.0);
                                if random_number <= self.pdr {
                                    let error_packet = create_error_packet(&packet, NackType::Dropped, fragment_index);
                                    let _ = self.forward_packet(error_packet);
                                } else {
                                    match self.forward_packet(packet.clone()) {
                                        Ok(()) => {},
                                        Err(error) => {
                                            let error_packet = create_error_packet(&packet, error, fragment_index);
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
                            Command::AddChannel(_, sender) => todo!(),
                            Command::RemoveChannel(_) => todo!(),
                            Command::Crash => todo!(),
                        }
                    }
                }
            }
        }
    }
}

fn create_error_packet(original_packet: &Packet, nack_type: NackType, fragment_index: u64) -> Packet {
    let hop_index = original_packet.routing_header.hop_index;
    let reverse_path = original_packet
        .routing_header
        .hops
        .split_at(hop_index + 1)
        .0
        .iter()
        .rev()
        .copied()
        // .map(|node| *node) // copied substitutes this map call
        .collect::<Vec<NodeId>>();

    let mut rng = rand::thread_rng();
    let new_session_id: u64 = rng.random();
    Packet {
        pack_type: PacketType::Nack(
            Nack {
                fragment_index,
                time_of_fail: Instant::now(),
                nack_type,
            }
        ),
        routing_header: SourceRoutingHeader {
            hop_index: 0,
            hops: reverse_path
        },
        session_id: new_session_id, // TODO: ask how session_id's are generated
    }
}

impl MyDrone {
    // TODO: ask what to do if a nack/ack packet contains invalid routing
    // Note: by protocol constraint, a drone cannot check hop_index field.
    // Therefore, you have to rely on other types of checks to verify whether a drone
    // is the destination of a packet and, if so, signal the logical error
    fn forward_packet(&self, mut packet: Packet) -> Result<(), NackType> {
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
                        Err(NackType::ErrorInRouting(*next_node))
                    }
                }
            },
            None => {
                // if the match expression returns None, it means that the current drone
                // is the designed destination
                Err(NackType::DestinationIsDrone)
            }
        }
    }


    fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
    }

    // fn remove_channel(...) {...}
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
