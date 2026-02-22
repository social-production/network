use libp2p::{Multiaddr, PeerId};
use sp_node::{Node, NodeConfig, NodeEvent};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::warn;

/// Commands sent from the TUI to the controller task.
pub enum ControlCommand {
    Start,
    Stop,
    Restart,
    Connect(Multiaddr),
    Disconnect(PeerId),
    /// Trigger active discovery; `None` means use the node's own port.
    Discover(Option<(u16, u16)>),
}

/// Messages sent from the controller task back to the TUI.
pub enum ControlEvent {
    NodeStarted { peer_id: String, listen_addr: String },
    NodeStopped,
    NodeEvent(NodeEvent),
    Error(String),
}

/// Manages the lifecycle of a [`Node`] in a background Tokio task.
pub struct NodeController {
    pub cmd_tx: mpsc::UnboundedSender<ControlCommand>,
    pub event_rx: mpsc::UnboundedReceiver<ControlEvent>,
}

impl NodeController {
    /// Spawn the controller task and return handles to communicate with it.
    pub fn spawn(config: NodeConfig) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<ControlCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ControlEvent>();

        tokio::spawn(run_controller(config, cmd_rx, event_tx));

        Self { cmd_tx, event_rx }
    }
}

async fn run_controller(
    config: NodeConfig,
    mut cmd_rx: mpsc::UnboundedReceiver<ControlCommand>,
    event_tx: mpsc::UnboundedSender<ControlEvent>,
) {
    // Channel used to send commands to a running node task.
    let mut node_cmd_tx: Option<mpsc::UnboundedSender<NodeCommand>> = None;
    let mut node_handle: Option<JoinHandle<()>> = None;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            ControlCommand::Start | ControlCommand::Restart => {
                // Stop any running node first.
                if let Some(tx) = node_cmd_tx.take() {
                    let _ = tx.send(NodeCommand::Stop);
                }
                if let Some(handle) = node_handle.take() {
                    handle.abort();
                    let _ = event_tx.send(ControlEvent::NodeStopped);
                }

                let ev_tx = event_tx.clone();
                let cfg = config.clone();

                let (n_cmd_tx, n_cmd_rx) = mpsc::unbounded_channel::<NodeCommand>();
                node_cmd_tx = Some(n_cmd_tx);

                node_handle = Some(tokio::spawn(run_node(cfg, n_cmd_rx, ev_tx)));
            }

            ControlCommand::Stop => {
                if let Some(tx) = node_cmd_tx.take() {
                    let _ = tx.send(NodeCommand::Stop);
                }
                if let Some(handle) = node_handle.take() {
                    handle.abort();
                    let _ = event_tx.send(ControlEvent::NodeStopped);
                }
            }

            ControlCommand::Connect(addr) => {
                if let Some(tx) = &node_cmd_tx {
                    let _ = tx.send(NodeCommand::Dial(addr));
                }
            }

            ControlCommand::Disconnect(peer_id) => {
                if let Some(tx) = &node_cmd_tx {
                    let _ = tx.send(NodeCommand::Disconnect(peer_id));
                }
            }

            ControlCommand::Discover(range) => {
                if let Some(tx) = &node_cmd_tx {
                    let _ = tx.send(NodeCommand::Discover(range));
                }
            }
        }
    }
}

/// Internal commands forwarded into the node task.
enum NodeCommand {
    Stop,
    Dial(Multiaddr),
    Disconnect(PeerId),
    Discover(Option<(u16, u16)>),
}

async fn run_node(
    config: NodeConfig,
    mut cmd_rx: mpsc::UnboundedReceiver<NodeCommand>,
    event_tx: mpsc::UnboundedSender<ControlEvent>,
) {
    match Node::new(config).await {
        Err(e) => {
            let _ = event_tx.send(ControlEvent::Error(e.to_string()));
        }
        Ok((mut node, mut node_events)) => {
            let peer_id = node.peer_id().to_string();

            // Brief pause so the swarm binds its port before we announce.
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;

            let _ = event_tx.send(ControlEvent::NodeStarted {
                peer_id: peer_id.clone(),
                listen_addr: String::new(), // updated when Listening event arrives
            });

            loop {
                tokio::select! {
                    Some(ctrl) = cmd_rx.recv() => {
                        match ctrl {
                            NodeCommand::Stop => break,
                            NodeCommand::Dial(addr) => {
                                if let Err(e) = node.dial(addr) {
                                    let _ = event_tx.send(ControlEvent::Error(e.to_string()));
                                }
                            }
                            NodeCommand::Disconnect(pid) => {
                                if let Err(e) = node.disconnect(pid) {
                                    let _ = event_tx.send(ControlEvent::Error(e.to_string()));
                                }
                            }
                            NodeCommand::Discover(range) => {
                                node.trigger_discovery(range);
                            }
                        }
                    }
                    Some(ev) = node_events.recv() => {
                        if event_tx.send(ControlEvent::NodeEvent(ev)).is_err() {
                            break;
                        }
                    }
                    _ = node.run() => {
                        break;
                    }
                }
            }

            warn!("Node task exiting");
            let _ = event_tx.send(ControlEvent::NodeStopped);
        }
    }
}
