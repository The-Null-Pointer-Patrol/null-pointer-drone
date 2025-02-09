use crossbeam_channel::Sender;
use wg_2024::network::NodeId;
use wg_2024::packet::Packet;
use crate::{MyDrone, State};

impl MyDrone {
    /// Sets `self.pdr` to the given `pdr` value.
    /// # Panics
    /// Panics if `pdr` is not in the valid range
    pub(crate) fn set_pdr(&mut self, pdr: f32) {
        assert!(
            (0f32..=1f32).contains(&pdr),
            "Tried to set an invalid pdr value of {pdr}, which is not in range (0.0..=1.0)"
        );
        self.pdr = pdr;
        log::info!("pdr set to {pdr}");
    }

    /// Adds a channel to send `Packet`s on
    /// # Panics
    /// Panics if the new channel id is the same as the drone's
    pub(crate) fn add_channel(&mut self, id: NodeId, sender: Sender<Packet>) {
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

    /// Removes a channel to send `Packet`s on
    /// # Panics
    /// Panics if there is no channel associated to `node_id`
    pub(crate) fn remove_channel(&mut self, node_id: NodeId) {
        match self.packet_send.remove(&node_id) {
            Some(_removed_channel) => {
                log::info!("Channel to {node_id} removed successfully");
            }
            None => {
                panic!("Cannot remove channel to {node_id}: it does not exist")
            }
        }
    }

    /// Updates the state of the drone
    pub(crate) fn set_state(&mut self, state: State) {
        self.state = state;
        log::info!("state set to {state:?}");
    }
}