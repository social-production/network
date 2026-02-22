use std::ops::RangeInclusive;

use crate::mode::NodeMode;
use sp_sync::SyncStrategy;

/// Controls which peer-discovery mechanism(s) the node uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryMode {
    /// Use only the Kademlia DHT for global peer discovery (default).
    KademliaDht,
    /// Use only local-network mDNS for peer discovery.
    Mdns,
    /// Use both Kademlia DHT and mDNS.
    Both,
}

impl Default for DiscoveryMode {
    fn default() -> Self {
        // Both enables mDNS for local/LAN peers and Kademlia for internet-wide
        // discovery.  Using KademliaDht alone would silently drop all mDNS
        // events, making local nodes invisible to each other.
        DiscoveryMode::Both
    }
}

/// Full configuration for a [`crate::Node`].
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// TCP port the node listens on.  Defaults to `51025`.
    pub port: u16,

    /// If `Some`, only peer addresses whose port falls within this range will
    /// be acted on during discovery.  `None` (the default) accepts all ports.
    pub discovery_port_range: Option<RangeInclusive<u16>>,

    /// Which peer-discovery mechanism(s) to use.
    pub discovery_mode: DiscoveryMode,

    /// Full participant vs. gossip-only operation.
    pub mode: NodeMode,

    /// Controls which blocks are synced from peers.
    pub sync_strategy: SyncStrategy,

    /// When `true` the binary embedding this node should suppress log output
    /// to stderr (e.g. redirect to a file) so the node runs silently.
    /// The library itself does not initialise a tracing subscriber; this flag
    /// is a signal to the host binary.
    pub quiet: bool,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            port: 51025,
            discovery_port_range: None,
            discovery_mode: DiscoveryMode::default(),
            mode: NodeMode::default(),
            sync_strategy: SyncStrategy::default(),
            quiet: false,
        }
    }
}

impl NodeConfig {
    /// Create a config for a gossip-only node on the default port.
    ///
    /// A gossip node relays transactions and block announcements without
    /// storing assets or contributing block verifications.  It is a
    /// lightweight option suitable for mobile or IoT deployments.
    pub fn gossip() -> Self {
        Self {
            mode: NodeMode::Gossip,
            ..Self::default()
        }
    }

    /// Create a config for a gossip-only node on a specific port.
    pub fn gossip_on_port(port: u16) -> Self {
        Self {
            port,
            mode: NodeMode::Gossip,
            ..Self::default()
        }
    }

    /// Create a full-node config on a specific port.
    pub fn on_port(port: u16) -> Self {
        Self {
            port,
            ..Self::default()
        }
    }

    /// Returns `true` if the given port is within the configured discovery
    /// port range (or if no range restriction is configured).
    pub fn port_allowed(&self, port: u16) -> bool {
        match &self.discovery_port_range {
            None => true,
            Some(range) => range.contains(&port),
        }
    }
}
