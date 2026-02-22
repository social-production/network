use serde::{Deserialize, Serialize};
use sp_blockchain::Block;
use sp_transaction::Transaction;

/// Topics used on the gossipsub overlay.
pub const TOPIC_TX: &str = "sp/tx";
pub const TOPIC_VERIFY: &str = "sp/verify";
pub const TOPIC_BLOCK: &str = "sp/block";

/// Messages sent over the gossipsub topics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GossipMessage {
    /// A new transaction broadcast to all peers.
    Transaction(Transaction),

    /// A peer signals that it has verified the block at `block_index`.
    BlockVerification {
        block_index: u64,
        peer_id: String,
    },

    /// A newly formed block broadcast to all peers.
    Block(Block),
}

/// Request/response codec for direct peer-to-peer block sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Request all blocks with index >= `from_index`.
    BlocksFrom { from_index: u64 },

    /// Request the current chain length (tip index) from a peer.
    ChainTip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    Blocks(Vec<Block>),
    ChainTip { tip_index: u64 },
}

/// Encode a [`GossipMessage`] to bytes for gossipsub.
pub fn encode_gossip(msg: &GossipMessage) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(msg)
}

/// Decode bytes from gossipsub into a [`GossipMessage`].
pub fn decode_gossip(bytes: &[u8]) -> Result<GossipMessage, bincode::Error> {
    bincode::deserialize(bytes)
}

/// Encode a [`SyncRequest`] for the request-response protocol.
pub fn encode_request(req: &SyncRequest) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(req)
}

/// Decode bytes into a [`SyncRequest`].
pub fn decode_request(bytes: &[u8]) -> Result<SyncRequest, bincode::Error> {
    bincode::deserialize(bytes)
}

/// Encode a [`SyncResponse`].
pub fn encode_response(resp: &SyncResponse) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(resp)
}

/// Decode bytes into a [`SyncResponse`].
pub fn decode_response(bytes: &[u8]) -> Result<SyncResponse, bincode::Error> {
    bincode::deserialize(bytes)
}
