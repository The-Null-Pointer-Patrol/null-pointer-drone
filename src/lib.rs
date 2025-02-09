use core::panic;
use crossbeam_channel::{select_biased, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use wg_2024::controller::{DroneCommand, DroneEvent};
use wg_2024::drone::Drone;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;

mod configuration;
mod packet_processing;
mod packet_sending;

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

    /// runs the drone loop, listening for events and packets, exits successfully if sent a
    /// `DroneCommand::Crash` after all references of the sender for its own receiver have been
    /// dropped
    ///
    /// # Panics
    /// - The `Sender<DroneCommand>` end of the simulation controller channel unexpectedly got dropped
    ///
    /// - There is no connected sender to the drone's receiver channel and no `DroneCommand::Crash` has been received
    ///
    /// - Tried to set an invalid pdr value of {`pdr`}, which is not in range (0.0..=1.0)
    ///
    /// - Cannot add a channel with the same `NodeId` of this drone (which is {})
    ///   You cannot connect the drone to itself or another drone with the same id
    ///
    /// - Cannot remove channel to {`node_id`}: it does not exist
    ///
    /// - empty routing header for packet {`packet`}
    ///
    /// - `hop_index` out of bounds: index {`current_index`} for hops {:?}
    ///
    /// - received packet with `hop_index` 0, which should be impossible
    ///   Refers to packets that are not a flood request, maybe this packet was supposed to be a **flood request**
    ///
    /// - flood request has no path trace
    ///
    /// - Cannot send packet {&packet} into channel {channel:?}. Error: {error:?}
    ///   when the crossbeam channel send returns an error
    ///
    /// - Cannot send event {&event} to simulation controller. Error: {error:?}"
    ///   same thing as above but for event
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
