use libp2p::{Multiaddr, PeerId};
use sp_blockchain::Block;
use sp_transaction::Transaction;

/// High-level events emitted by a running [`Node`] that callers (e.g. the
/// TUI) can subscribe to via a channel.
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// A new peer has connected.
    PeerConnected(PeerId),

    /// A peer has disconnected.
    PeerDisconnected(PeerId),

    /// A peer was discovered by mDNS or Kademlia but is not yet connected.
    PeerDiscovered {
        peer_id: PeerId,
        addrs: Vec<Multiaddr>,
    },

    /// A new transaction has arrived via gossip.
    TransactionReceived(Transaction),

    /// A new block has been broadcast by a peer.
    BlockReceived(Block),

    /// A block has been verified by enough peers and is now finalised.
    BlockFinalised { block_index: u64 },

    /// The local chain has been replaced by a longer remote chain.
    ChainSynced { new_length: usize },

    /// The node is now listening on the given address.
    Listening(Multiaddr),
}
