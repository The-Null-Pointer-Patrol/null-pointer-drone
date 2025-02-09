use wg_2024::network::{NodeId, SourceRoutingHeader};
use wg_2024::packet::{FloodRequest, FloodResponse, NackType, NodeType, Packet, PacketType};
use crate::MyDrone;

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
