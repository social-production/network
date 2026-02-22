use std::collections::HashMap;

use futures::StreamExt;
use libp2p::{
    gossipsub::IdentTopic,
    request_response::Message as RrMessage,
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use sp_blockchain::Blockchain;
use sp_sync::SyncManager;
use sp_transaction::Transaction;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    behaviour::{build_behaviour, SpBehaviour, SpBehaviourEvent},
    config::{DiscoveryMode, NodeConfig},
    error::NodeError,
    event::NodeEvent,
    mode::NodeMode,
    protocol::{
        decode_gossip, decode_request, encode_gossip, encode_response, GossipMessage, SyncRequest,
        SyncResponse, TOPIC_BLOCK, TOPIC_TX, TOPIC_VERIFY,
    },
};

/// Maximum number of pending transactions before they are automatically batched
/// into a new block.
const BLOCK_BATCH_SIZE: usize = 10;

/// The Social Production P2P node.
///
/// Wraps a libp2p [`Swarm`] and exposes a simple async API for:
/// - Connecting to the network
/// - Broadcasting and receiving transactions
/// - Block formation, gossip, and verification
/// - Chain sync with peers
/// - Peer management (connect, disconnect, list discovered/connected)
pub struct Node {
    swarm: Swarm<SpBehaviour>,
    local_peer_id: PeerId,
    mode: NodeMode,
    blockchain: Blockchain,
    sync_manager: SyncManager,
    pending_transactions: Vec<Transaction>,
    event_tx: mpsc::UnboundedSender<NodeEvent>,
    /// Peers found via discovery but not yet connected.
    discovered_peers: HashMap<PeerId, Vec<Multiaddr>>,
    /// Currently connected peers and their known addresses.
    connected_peers_map: HashMap<PeerId, Vec<Multiaddr>>,
    /// Controls which discovery events to act on.
    discovery_mode: DiscoveryMode,
    /// Optional port range filter applied to discovered peer addresses.
    discovery_port_range: Option<std::ops::RangeInclusive<u16>>,
    /// The port this node is listening on (retained for future use).
    #[allow(dead_code)]
    port: u16,
}

impl Node {
    /// Create and configure a new node from a [`NodeConfig`].
    ///
    /// Returns the node together with a receiver for [`NodeEvent`]s that the
    /// calling application can process independently.
    pub async fn new(
        config: NodeConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<NodeEvent>), NodeError> {
        let keypair = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = keypair.public().to_peer_id();

        info!("Local peer id: {local_peer_id}");

        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.port)
            .parse()
            .map_err(|e: libp2p::multiaddr::Error| NodeError::Transport(e.to_string()))?;

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| NodeError::Transport(e.to_string()))?
            .with_behaviour(|_| build_behaviour(&keypair))
            .map_err(|e| NodeError::Transport(e.to_string()))?
            .build();

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut node = Self {
            swarm,
            local_peer_id,
            mode: config.mode,
            blockchain: Blockchain::new(),
            sync_manager: SyncManager::new(config.sync_strategy),
            pending_transactions: Vec::new(),
            event_tx,
            discovered_peers: HashMap::new(),
            connected_peers_map: HashMap::new(),
            discovery_mode: config.discovery_mode,
            discovery_port_range: config.discovery_port_range,
            port: config.port,
        };

        node.swarm
            .listen_on(listen_addr)
            .map_err(|e| NodeError::Transport(e.to_string()))?;

        Ok((node, event_rx))
    }

    /// Return the local [`PeerId`].
    pub fn peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Provide read access to the local blockchain.
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    /// Return a snapshot of currently connected peers and their known addresses.
    pub fn connected_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.connected_peers_map
            .iter()
            .map(|(pid, addrs)| (*pid, addrs.clone()))
            .collect()
    }

    /// Return a snapshot of discovered-but-not-yet-connected peers.
    pub fn discovered_peers(&self) -> Vec<(PeerId, Vec<Multiaddr>)> {
        self.discovered_peers
            .iter()
            .map(|(pid, addrs)| (*pid, addrs.clone()))
            .collect()
    }

    /// Dial a remote peer by multiaddr.
    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), NodeError> {
        self.swarm
            .dial(addr)
            .map_err(|e| NodeError::Transport(e.to_string()))
    }

    /// Disconnect from a connected peer.
    pub fn disconnect(&mut self, peer_id: PeerId) -> Result<(), NodeError> {
        self.swarm
            .disconnect_peer_id(peer_id)
            .map_err(|_| NodeError::Transport(format!("peer {peer_id} not connected")))
    }

    /// Trigger an active discovery scan using both mDNS and Kademlia.
    ///
    /// `port_range` — when `Some((start, end))` only peer addresses whose port
    /// falls in that range are accepted.  When `None` the port filter is cleared
    /// so peers on any port are accepted (the "search on the node's own network"
    /// default from the PLAN).
    ///
    /// Kademlia `bootstrap()` refreshes the routing table and triggers
    /// `RoutingUpdated` events → `PeerDiscovered` events to the TUI.
    /// mDNS runs continuously in the background and surfaces results as soon as
    /// `discovery_mode` allows them through.
    pub fn trigger_discovery(&mut self, port_range: Option<(u16, u16)>) {
        self.discovery_port_range = port_range.map(|(start, end)| start..=end);
        // Ensure both mDNS and Kademlia results flow through.
        self.discovery_mode = DiscoveryMode::Both;
        let _ = self.swarm.behaviour_mut().kademlia.bootstrap();
    }

    /// Broadcast a transaction to all connected peers via gossipsub.
    pub fn broadcast_transaction(&mut self, tx: Transaction) -> Result<(), NodeError> {
        let msg = GossipMessage::Transaction(tx.clone());
        let bytes =
            encode_gossip(&msg).map_err(|e| NodeError::Serialisation(e.to_string()))?;

        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(TOPIC_TX), bytes)
            .map_err(|e| NodeError::Gossipsub(e.to_string()))?;

        self.pending_transactions.push(tx);
        self.maybe_form_block()?;

        Ok(())
    }

    /// Seal pending transactions into a block and broadcast it.
    pub fn form_block(&mut self) -> Result<(), NodeError> {
        if self.pending_transactions.is_empty() {
            return Err(NodeError::NoPendingTransactions);
        }

        let txs = std::mem::take(&mut self.pending_transactions);
        let block = self.blockchain.add_block(txs)?;
        let block_index = block.index;
        let block_clone = block.clone();

        info!("Formed block #{block_index}");

        let msg = GossipMessage::Block(block_clone);
        if let Ok(bytes) = encode_gossip(&msg) {
            let _ = self
                .swarm
                .behaviour_mut()
                .gossipsub
                .publish(IdentTopic::new(TOPIC_BLOCK), bytes);
        }

        if self.mode == NodeMode::Full {
            self.send_verification(block_index)?;
        }

        Ok(())
    }

    /// Send a block verification for `block_index` to all peers.
    pub fn send_verification(&mut self, block_index: u64) -> Result<(), NodeError> {
        let peer_id_str = self.local_peer_id.to_string();
        let msg = GossipMessage::BlockVerification {
            block_index,
            peer_id: peer_id_str.clone(),
        };
        let bytes =
            encode_gossip(&msg).map_err(|e| NodeError::Serialisation(e.to_string()))?;

        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(TOPIC_VERIFY), bytes)
            .map_err(|e| NodeError::Gossipsub(e.to_string()))?;

        let finalised = self.blockchain.verify_block(block_index, peer_id_str)?;

        if finalised {
            let _ = self
                .event_tx
                .send(NodeEvent::BlockFinalised { block_index });
        }

        Ok(())
    }

    /// Run the node event loop.  This future runs until cancelled.
    pub async fn run(&mut self) {
        loop {
            let event = self.swarm.select_next_some().await;
            self.handle_swarm_event(event).await;
        }
    }

    /// Run the node event loop with automatic periodic discovery.
    ///
    /// Triggers an initial discovery scan immediately on entry, then repeats
    /// every `interval`.  The loop runs until cancelled (e.g. via Ctrl-C).
    pub async fn run_with_periodic_discovery(&mut self, interval: std::time::Duration) {
        use tokio::time;

        // Kick off an immediate scan before the first interval tick.
        self.trigger_discovery(None);

        let mut ticker = time::interval(interval);
        // Skip ticks that fire while we're busy handling swarm events so we
        // don't queue up a backlog of discovery calls.
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        // Consume the first (immediate) tick so the next fires after `interval`.
        ticker.tick().await;

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
                _ = ticker.tick() => {
                    self.trigger_discovery(None);
                }
            }
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Filter peer addresses by the configured discovery port range.
    fn filter_addrs(&self, addrs: Vec<Multiaddr>) -> Vec<Multiaddr> {
        match &self.discovery_port_range {
            None => addrs,
            Some(range) => addrs
                .into_iter()
                .filter(|addr| {
                    addr_port(addr)
                        .map(|p| range.contains(&p))
                        .unwrap_or(false)
                })
                .collect(),
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<SpBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {address}");
                let _ = self.event_tx.send(NodeEvent::Listening(address));
            }

            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!("Connected to {peer_id}");
                let addr = endpoint.get_remote_address().clone();
                // Move from discovered → connected.
                self.discovered_peers.remove(&peer_id);
                self.connected_peers_map
                    .entry(peer_id)
                    .or_default()
                    .push(addr);
                let _ = self.event_tx.send(NodeEvent::PeerConnected(peer_id));
                self.request_chain_tip(peer_id);
            }

            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!("Disconnected from {peer_id}");
                self.connected_peers_map.remove(&peer_id);
                let _ = self.event_tx.send(NodeEvent::PeerDisconnected(peer_id));
            }

            SwarmEvent::Behaviour(SpBehaviourEvent::Gossipsub(
                libp2p::gossipsub::Event::Message { message, .. },
            )) => {
                self.handle_gossip_message(&message.data).await;
            }

            SwarmEvent::Behaviour(SpBehaviourEvent::Mdns(
                libp2p::mdns::Event::Discovered(peers),
            )) => {
                // Respect discovery mode — ignore mDNS if Kademlia-only.
                if self.discovery_mode == DiscoveryMode::KademliaDht {
                    return;
                }
                let mut by_peer: HashMap<PeerId, Vec<Multiaddr>> = HashMap::new();
                for (peer_id, addr) in peers {
                    by_peer.entry(peer_id).or_default().push(addr);
                }
                for (peer_id, addrs) in by_peer {
                    if self.connected_peers_map.contains_key(&peer_id) {
                        continue;
                    }
                    let filtered = self.filter_addrs(addrs.clone());
                    if filtered.is_empty() && self.discovery_port_range.is_some() {
                        continue;
                    }
                    let kept = if filtered.is_empty() { addrs } else { filtered };
                    // Add to Kademlia routing table regardless.
                    for addr in &kept {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr.clone());
                    }
                    let entry = self.discovered_peers.entry(peer_id).or_default();
                    for addr in &kept {
                        if !entry.contains(addr) {
                            entry.push(addr.clone());
                        }
                    }
                    let _ = self.event_tx.send(NodeEvent::PeerDiscovered {
                        peer_id,
                        addrs: kept,
                    });
                }
            }

            SwarmEvent::Behaviour(SpBehaviourEvent::Kademlia(
                libp2p::kad::Event::RoutingUpdated { peer, addresses, .. },
            )) => {
                // Respect discovery mode — ignore Kademlia if mDNS-only.
                if self.discovery_mode == DiscoveryMode::Mdns {
                    return;
                }
                if self.connected_peers_map.contains_key(&peer) {
                    return;
                }
                let addrs: Vec<Multiaddr> = addresses.into_vec();
                let filtered = self.filter_addrs(addrs.clone());
                let kept = if filtered.is_empty() && self.discovery_port_range.is_some() {
                    return;
                } else if filtered.is_empty() {
                    addrs
                } else {
                    filtered
                };
                let entry = self.discovered_peers.entry(peer).or_default();
                for addr in &kept {
                    if !entry.contains(addr) {
                        entry.push(addr.clone());
                    }
                }
                let _ = self.event_tx.send(NodeEvent::PeerDiscovered {
                    peer_id: peer,
                    addrs: kept,
                });
            }

            // When a peer sends us its Identify info, register its listen
            // addresses in the Kademlia routing table.  Without this step,
            // kademlia.bootstrap() has an empty table and can't reach anyone.
            SwarmEvent::Behaviour(SpBehaviourEvent::Identify(
                libp2p::identify::Event::Received { peer_id, info, .. },
            )) => {
                for addr in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }

            // Disconnect peers that fail to respond to pings — they are
            // considered dead.  The resulting ConnectionClosed event handles
            // removing them from connected_peers_map and emitting
            // NodeEvent::PeerDisconnected.
            SwarmEvent::Behaviour(SpBehaviourEvent::Ping(libp2p::ping::Event {
                peer,
                result: Err(_),
                ..
            })) => {
                debug!("Ping failed for {peer}, disconnecting");
                let _ = self.swarm.disconnect_peer_id(peer);
            }

            SwarmEvent::Behaviour(SpBehaviourEvent::RequestResponse(
                libp2p::request_response::Event::Message { peer, message, .. },
            )) => {
                self.handle_request_response(peer, message).await;
            }

            _ => {}
        }
    }

    async fn handle_gossip_message(&mut self, data: &[u8]) {
        match decode_gossip(data) {
            Ok(GossipMessage::Transaction(tx)) => {
                debug!("Received transaction {}", tx.id);
                let _ = self.event_tx.send(NodeEvent::TransactionReceived(tx.clone()));
                self.pending_transactions.push(tx);
                let _ = self.maybe_form_block();
            }

            Ok(GossipMessage::Block(block)) => {
                let block_index = block.index;
                debug!("Received block #{block_index}");
                let _ = self.event_tx.send(NodeEvent::BlockReceived(block));

                if self.mode == NodeMode::Full {
                    let _ = self.send_verification(block_index);
                }
            }

            Ok(GossipMessage::BlockVerification { block_index, peer_id }) => {
                match self.blockchain.verify_block(block_index, peer_id) {
                    Ok(true) => {
                        info!("Block #{block_index} finalised");
                        let _ = self
                            .event_tx
                            .send(NodeEvent::BlockFinalised { block_index });
                    }
                    Ok(false) => {}
                    Err(e) => warn!("verify_block error: {e}"),
                }
            }

            Err(e) => warn!("Failed to decode gossip message: {e}"),
        }
    }

    async fn handle_request_response(
        &mut self,
        _peer: PeerId,
        message: RrMessage<Vec<u8>, Vec<u8>>,
    ) {
        match message {
            RrMessage::Request { request, channel, .. } => {
                let response = match decode_request(&request) {
                    Ok(SyncRequest::ChainTip) => {
                        let tip = self.blockchain.tip().index;
                        encode_response(&SyncResponse::ChainTip { tip_index: tip })
                    }
                    Ok(SyncRequest::BlocksFrom { from_index }) => {
                        let blocks = self.blockchain.blocks_from(from_index).to_vec();
                        encode_response(&SyncResponse::Blocks(blocks))
                    }
                    Err(e) => {
                        warn!("Failed to decode sync request: {e}");
                        return;
                    }
                };

                if let Ok(bytes) = response {
                    let _ = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, bytes);
                }
            }

            RrMessage::Response { response, .. } => {
                self.apply_sync_response(&response).await;
            }
        }
    }

    async fn apply_sync_response(&mut self, data: &[u8]) {
        match crate::protocol::decode_response(data) {
            Ok(SyncResponse::ChainTip { tip_index }) => {
                let local_tip = self.blockchain.tip().index;
                if tip_index > local_tip {
                    debug!("Peer tip ({tip_index}) > local ({local_tip}), requesting blocks");
                    let peer = self.swarm.connected_peers().next().copied();
                    if let Some(peer) = peer {
                        if let Ok(bytes) = crate::protocol::encode_request(
                            &SyncRequest::BlocksFrom { from_index: local_tip + 1 },
                        ) {
                            self.swarm
                                .behaviour_mut()
                                .request_response
                                .send_request(&peer, bytes);
                        }
                    }
                }
            }

            Ok(SyncResponse::Blocks(remote_blocks)) => {
                let remote_chain = Blockchain::new();
                for block in remote_blocks {
                    if block.index > 0 {
                        let _ = self.sync_manager.record_download(&block);
                    }
                }
                if self.blockchain.sync_from(&remote_chain) {
                    let new_length = self.blockchain.len();
                    info!("Chain synced to length {new_length}");
                    let _ = self.event_tx.send(NodeEvent::ChainSynced { new_length });
                }
            }

            Err(e) => warn!("Failed to decode sync response: {e}"),
        }
    }

    fn request_chain_tip(&mut self, peer: PeerId) {
        if let Ok(bytes) = crate::protocol::encode_request(&SyncRequest::ChainTip) {
            self.swarm
                .behaviour_mut()
                .request_response
                .send_request(&peer, bytes);
        }
    }

    fn maybe_form_block(&mut self) -> Result<(), NodeError> {
        if self.pending_transactions.len() >= BLOCK_BATCH_SIZE {
            self.form_block()?;
        }
        Ok(())
    }
}

/// Extract the TCP/UDP port from a multiaddr, if present.
fn addr_port(addr: &Multiaddr) -> Option<u16> {
    use libp2p::multiaddr::Protocol;
    for proto in addr.iter() {
        match proto {
            Protocol::Tcp(port) | Protocol::Udp(port) => return Some(port),
            _ => {}
        }
    }
    None
}
