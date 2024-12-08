use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, Packet, PacketType},
};

pub struct PacketBuilder {
    pub routing_header: SourceRoutingHeader,
    pub session_id: u64,
    pub pack_type: PacketType,
}

impl PacketBuilder {
    /// sets session_id to 0 and hop_index to 1 by default
    pub fn new(pack_type: PacketType, hops: Vec<NodeId>) -> PacketBuilder {
        PacketBuilder {
            routing_header: SourceRoutingHeader { hops, hop_index: 1 },
            session_id: 0,
            pack_type,
        }
    }

    /// sets session_id to 0, hop_index to 1, creates a fragment with index 0, total_n_fragments 1,
    /// length of 128 and data vector full of zeros
    pub fn new_fragment(hops: Vec<NodeId>) -> PacketBuilder {
        PacketBuilder::new(
            PacketType::MsgFragment(Fragment {
                fragment_index: 0,
                total_n_fragments: 1,
                length: 128,
                data: [0; 128],
            }),
            hops,
        )
    }

    /// sets session_id to 0, hop_index to 1, creates a nack with given type and fragment_index 0
    pub fn new_nack(hops: Vec<NodeId>, nack_type: NackType) -> PacketBuilder {
        PacketBuilder::new(
            PacketType::Nack(Nack {
                fragment_index: 0,
                nack_type,
            }),
            hops,
        )
    }

    /// sets session_id to 0, hop_index to 1, creates an ack with fragment_index 0
    pub fn new_ack(hops: Vec<NodeId>) -> PacketBuilder {
        PacketBuilder::new(PacketType::Ack(Ack { fragment_index: 0 }), hops)
    }

    /// sets session_id to 0, hop_index to 1, creates a flood response with flood_id 0 and empty
    /// path trace
    pub fn new_floodresp(hops: Vec<NodeId>) -> PacketBuilder {
        PacketBuilder::new(
            PacketType::FloodResponse(FloodResponse {
                flood_id: 0,
                path_trace: vec![],
            }),
            hops,
        )
    }

    /// sets session_id to 0, hop_index to 1, creates a flood request with flood_id 0 empty
    /// path trace, and initiator_id as hops[0]
    pub fn new_floodreq(hops: Vec<NodeId>) -> PacketBuilder {
        PacketBuilder::new(
            PacketType::FloodRequest(FloodRequest {
                flood_id: 0,
                path_trace: vec![],
                initiator_id: hops[0],
            }),
            hops,
        )
    }

    pub fn hop_index(&mut self, hop: usize) {
        self.routing_header.hop_index = hop;
    }
    pub fn session_id(&mut self, sid: u64) {
        self.session_id = sid;
    }
    pub fn build(self) -> Packet {
        Packet {
            routing_header: self.routing_header,
            session_id: self.session_id,
            pack_type: self.pack_type,
        }
    }
}
