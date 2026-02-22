/// Controls how much work a node does on behalf of the network.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NodeMode {
    /// Full participant: validates blocks, stores assets, contributes
    /// verifications so that blocks can be finalised.
    #[default]
    Full,

    /// Gossip-only: relays transactions and block announcements to peers but
    /// does not store assets and does not send verification messages.
    /// Useful for lightweight mobile/IoT deployments.
    Gossip,
}
