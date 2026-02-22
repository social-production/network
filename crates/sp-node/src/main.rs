use std::time::Duration;

use clap::{Parser, ValueEnum};
use sp_node::{DiscoveryMode, Node, NodeConfig, NodeMode};
use sp_sync::SyncStrategy;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Social Production Network node daemon.
#[derive(Parser, Debug)]
#[command(
    name = "sp-node",
    version,
    about = "Social Production Network P2P node",
    long_about = "Runs a Social Production Network P2P node that participates \
                  in the gossip network, verifies blocks, and syncs the blockchain."
)]
struct Cli {
    /// TCP port to listen on.
    #[arg(short, long, default_value_t = 51025, env = "SPN_PORT")]
    port: u16,

    /// Node operation mode.
    #[arg(short, long, default_value = "full", env = "SPN_MODE")]
    mode: CliMode,

    /// Peer-discovery mechanism.
    #[arg(short, long, default_value = "kademlia", env = "SPN_DISCOVERY")]
    discovery: CliDiscovery,

    /// Sync strategy.
    #[arg(short, long, default_value = "on-demand", env = "SPN_SYNC")]
    sync: CliSync,

    /// Minimum port for discovery filtering (inclusive). Omit to accept all ports.
    #[arg(long, env = "SPN_DISCOVERY_PORT_MIN")]
    discovery_port_min: Option<u16>,

    /// Maximum port for discovery filtering (inclusive). Omit to accept all ports.
    #[arg(long, env = "SPN_DISCOVERY_PORT_MAX")]
    discovery_port_max: Option<u16>,

    /// How often (in seconds) to re-run peer discovery. Default: 60.
    #[arg(long, default_value_t = 60, env = "SPN_DISCOVERY_INTERVAL")]
    discovery_interval: u64,

    /// Suppress log output to stderr (run silently).
    #[arg(short, long, default_value_t = false, env = "SPN_QUIET")]
    quiet: bool,
}

#[derive(ValueEnum, Debug, Clone)]
enum CliMode {
    /// Full participant: validates blocks and sends verifications.
    Full,
    /// Gossip-only: relays messages without storing assets or verifying blocks.
    Gossip,
}

#[derive(ValueEnum, Debug, Clone)]
enum CliDiscovery {
    /// Kademlia distributed hash table (default, works across the internet).
    Kademlia,
    /// mDNS local-network discovery only.
    Mdns,
    /// Both Kademlia and mDNS.
    Both,
}

#[derive(ValueEnum, Debug, Clone)]
enum CliSync {
    /// Sync blocks only when explicitly requested.
    #[value(name = "on-demand")]
    OnDemand,
    /// Sync all blocks (no restriction).
    All,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let log_filter = if cli.quiet {
        EnvFilter::new("off")
    } else {
        EnvFilter::from_default_env().add_directive("sp_node=info".parse()?)
    };
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    let discovery_port_range = match (cli.discovery_port_min, cli.discovery_port_max) {
        (Some(min), Some(max)) => Some(min..=max),
        (Some(min), None) => Some(min..=u16::MAX),
        (None, Some(max)) => Some(0..=max),
        (None, None) => None,
    };

    let config = NodeConfig {
        port: cli.port,
        mode: match cli.mode {
            CliMode::Full => NodeMode::Full,
            CliMode::Gossip => NodeMode::Gossip,
        },
        discovery_mode: match cli.discovery {
            CliDiscovery::Kademlia => DiscoveryMode::KademliaDht,
            CliDiscovery::Mdns => DiscoveryMode::Mdns,
            CliDiscovery::Both => DiscoveryMode::Both,
        },
        sync_strategy: match cli.sync {
            CliSync::OnDemand => SyncStrategy::OnDemand,
            CliSync::All => SyncStrategy::OnDemand,
        },
        discovery_port_range,
        quiet: cli.quiet,
    };

    let discovery_interval = Duration::from_secs(cli.discovery_interval);

    info!(
        port = config.port,
        mode = ?config.mode,
        discovery = ?config.discovery_mode,
        discovery_interval_secs = cli.discovery_interval,
        "Starting Social Production node"
    );

    let (mut node, mut events) = Node::new(config).await?;

    info!("Peer id: {}", node.peer_id());

    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            info!("NodeEvent: {event:?}");
        }
    });

    // Auto-discover on startup and repeat every `discovery_interval`.
    node.run_with_periodic_discovery(discovery_interval).await;

    Ok(())
}
