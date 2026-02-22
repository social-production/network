mod app;
mod controller;
mod events;
mod peers_store;
mod ui;

use std::{io, path::PathBuf, time::{Duration, Instant}};

use app::{App, ContentView, NodeState};
use controller::{ControlCommand, ControlEvent, NodeController};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::TuiEvent;
use ratatui::{backend::CrosstermBackend, Terminal};
use sp_node::{NodeConfig, NodeEvent};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Redirect all logs to a file so they never bleed onto the TUI screen.
    let log_path = spn_log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sp_node=info".parse()?))
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_tui(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(200);
    let discover_interval = Duration::from_secs(60);
    let mut last_discovery = Instant::now();

    let config = NodeConfig { quiet: true, ..NodeConfig::default() };
    let controller = NodeController::spawn(config);
    let cmd_tx = controller.cmd_tx;
    let mut event_rx = controller.event_rx;

    // Auto-start: kick off the node immediately on launch.
    app.node_state = NodeState::Starting;
    app.push_traffic("Auto-starting node…");
    let _ = cmd_tx.send(ControlCommand::Start);

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Drain controller events (non-blocking).
        while let Ok(ctrl_ev) = event_rx.try_recv() {
            handle_controller_event(&mut app, &cmd_tx, ctrl_ev);
        }

        match events::next_event(tick_rate)? {
            TuiEvent::Key(key) => {
                // Ctrl-C always quits.
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    if app.node_state == NodeState::Running {
                        let _ = cmd_tx.send(ControlCommand::Stop);
                    }
                    app.should_quit = true;
                    break;
                }

                match key.code {
                    KeyCode::Enter => {
                        let raw = app.input.trim().to_string();
                        app.input.clear();
                        app.reset_history_nav();
                        app.clear_output();
                        if !raw.is_empty() {
                            app.push_history(raw.clone());
                        }
                        execute_command(&mut app, &cmd_tx, &raw);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    // ↑/↓ navigate command history in the input prompt.
                    KeyCode::Up => {
                        app.history_prev();
                    }
                    KeyCode::Down => {
                        app.history_next();
                    }
                    // Page Up/Down scroll the traffic (or active) view.
                    KeyCode::PageUp => {
                        app.scroll_traffic_up();
                    }
                    KeyCode::PageDown => {
                        app.scroll_traffic_down();
                    }
                    KeyCode::Char(c) => {
                        // Typing a character exits history navigation.
                        app.reset_history_nav();
                        app.input.push(c);
                    }
                    _ => {}
                }
            }
            TuiEvent::Tick => {
                // Periodically re-run discovery to pick up new peers.
                if app.node_state == NodeState::Running
                    && last_discovery.elapsed() >= discover_interval
                {
                    last_discovery = Instant::now();
                    let _ = cmd_tx.send(ControlCommand::Discover(None));
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Parse and execute a slash command entered by the user.
fn execute_command(
    app: &mut App,
    cmd_tx: &tokio::sync::mpsc::UnboundedSender<ControlCommand>,
    raw: &str,
) {
    let parts: Vec<&str> = raw.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).copied().unwrap_or("").trim();

    match cmd {
        "/start" => {
            if app.node_state == NodeState::Stopped {
                app.node_state = NodeState::Starting;
                app.push_traffic("Starting node…");
                let _ = cmd_tx.send(ControlCommand::Start);
            } else {
                app.set_output("Node is already running. Use /stop or /restart.");
            }
        }

        "/stop" => {
            if app.node_state == NodeState::Running {
                app.push_traffic("Stopping node…");
                let _ = cmd_tx.send(ControlCommand::Stop);
            } else {
                app.set_output("Node is not running.");
            }
        }

        "/restart" => {
            app.node_state = NodeState::Restarting;
            app.push_traffic("Restarting node…");
            let _ = cmd_tx.send(ControlCommand::Restart);
        }

        "/traffic" => {
            app.view = ContentView::Traffic;
        }

        "/status" => {
            app.view = ContentView::Status;
            app.status.peers_connected = app.connected_peers.len();
            app.status.peers_discovered = app.discovered_peers.len();
        }

        "/discover" => {
            let port_range = if arg.is_empty() {
                None // controller will use the node's own port
            } else {
                match parse_port_range(arg) {
                    Some(range) => Some(range),
                    None => {
                        app.set_output("Usage: /discover [<start port>-<end port>]");
                        return;
                    }
                }
            };
            let desc = match port_range {
                None => "Discovering peers on node port…".to_string(),
                Some((s, e)) => format!("Discovering peers on ports {s}–{e}…"),
            };
            app.push_traffic(desc);
            app.view = ContentView::Discovered;
            let _ = cmd_tx.send(ControlCommand::Discover(port_range));
        }

        "/connected" => {
            app.view = ContentView::Connected;
            app.push_traffic("Switched to connected peers view");
        }

        "/disconnect" => {
            if arg.is_empty() {
                app.set_output("Usage: /disconnect <node-id>  or  /disconnect <ip>:<port>");
            } else if let Ok(peer_id) = arg.parse::<libp2p::PeerId>() {
                // Argument is a bare peer-id.
                app.push_traffic(format!("Disconnecting from {}", &arg[..arg.len().min(20)]));
                let _ = cmd_tx.send(ControlCommand::Disconnect(peer_id));
            } else if let Some(addr_str) = parse_ip_port(arg).or_else(|| {
                // Also accept a raw multiaddr like /ip4/1.2.3.4/tcp/1234
                if arg.starts_with('/') { Some(arg.to_string()) } else { None }
            }) {
                // Argument looks like ip:port or a multiaddr — look up the peer by address.
                match find_peer_by_addr(&app.connected_peers, &addr_str) {
                    Some(peer_id) => {
                        app.push_traffic(format!("Disconnecting from {arg}"));
                        let _ = cmd_tx.send(ControlCommand::Disconnect(peer_id));
                    }
                    None => {
                        app.set_output(format!("No connected peer found at {arg}"));
                    }
                }
            } else {
                app.set_output("Invalid argument. Use a peer-id or ip:port.");
            }
        }

        "/connect" => {
            if arg.is_empty() {
                app.set_output("Usage: /connect <ip>:<port>");
            } else {
                // Accept both /ip4/... multiaddr syntax and plain ip:port.
                let multiaddr_str = if arg.starts_with('/') {
                    arg.to_string()
                } else {
                    match parse_ip_port(arg) {
                        Some(m) => m,
                        None => {
                            app.set_output("Invalid address. Use ip:port or /ip4/x.x.x.x/tcp/port");
                            return;
                        }
                    }
                };
                match multiaddr_str.parse::<libp2p::Multiaddr>() {
                    Ok(addr) => {
                        app.push_traffic(format!("Connecting to {multiaddr_str}"));
                        let _ = cmd_tx.send(ControlCommand::Connect(addr));
                    }
                    Err(_) => {
                        app.set_output("Could not parse address as multiaddr.");
                    }
                }
            }
        }

        "/help" => {
            app.view = ContentView::Traffic;
            for line in [
                "─── available commands ─────────────────────────────────────",
                "/start                       start the node",
                "/stop                        stop the node",
                "/restart                     restart the node",
                "/traffic                     see the node's traffic",
                "/status                      see the node's status",
                "/discover [start-end]        discover peers (internet-wide Kademlia scan)",
                "/connected                   see nodes currently connected",
                "/connect <ip>:<port>         connect to a node",
                "/disconnect <node id>        disconnect from a node by peer-id",
                "/disconnect <ip>:<port>      disconnect from a node by address",
                "/help                        show this help",
                "/quit                        quit spn",
                "keys: ↑/↓ history · PgUp/PgDn scroll · Ctrl-C quit",
                "────────────────────────────────────────────────────────────",
            ] {
                app.push_traffic(line);
            }
        }

        "/quit" | "/exit" => {
            if app.node_state == NodeState::Running {
                let _ = cmd_tx.send(ControlCommand::Stop);
            }
            app.should_quit = true;
        }

        "" => {}

        other => {
            app.set_output(format!("Unknown command: {other}  (try /help)"));
        }
    }
}

/// Handle events arriving from the node controller task.
fn handle_controller_event(
    app: &mut App,
    cmd_tx: &tokio::sync::mpsc::UnboundedSender<ControlCommand>,
    ev: ControlEvent,
) {
    match ev {
        ControlEvent::NodeStarted { peer_id, listen_addr } => {
            app.node_state = NodeState::Running;
            app.status.peer_id = peer_id.clone();
            if !listen_addr.is_empty() {
                app.status.listen_addr = listen_addr.clone();
            }
            app.push_traffic(format!("Node started  peer {peer_id}"));

            // Auto-connect to previously known peers.
            let stored = peers_store::load();
            if !stored.is_empty() {
                app.push_traffic(format!(
                    "Reconnecting to {} stored peer(s)…",
                    stored.len()
                ));
                for addr_str in stored {
                    if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
                        let _ = cmd_tx.send(ControlCommand::Connect(addr));
                    }
                }
            }
        }

        ControlEvent::NodeStopped => {
            app.node_state = NodeState::Stopped;
            app.connected_peers.clear();
            app.status.peers_connected = 0;
            app.push_traffic("Node stopped");
        }

        ControlEvent::NodeEvent(node_ev) => match node_ev {
            NodeEvent::Listening(addr) => {
                app.status.listen_addr = addr.to_string();
                app.push_traffic(format!("Listening on {addr}"));
            }
            NodeEvent::PeerConnected(pid) => {
                let pid_str = pid.to_string();
                // Move from discovered → connected.
                app.discovered_peers.retain(|(id, _)| id != &pid_str);
                if !app.connected_peers.iter().any(|(id, _)| id == &pid_str) {
                    app.connected_peers.push((pid_str.clone(), Vec::new()));
                }
                app.status.peers_connected = app.connected_peers.len();
                app.status.peers_discovered = app.discovered_peers.len();
                app.push_traffic(format!("Peer connected: {pid_str}"));
            }
            NodeEvent::PeerDisconnected(pid) => {
                let pid_str = pid.to_string();
                app.connected_peers.retain(|(id, _)| id != &pid_str);
                app.status.peers_connected = app.connected_peers.len();
                app.push_traffic(format!("Peer disconnected: {pid_str}"));
            }
            NodeEvent::PeerDiscovered { peer_id, addrs } => {
                let pid_str = peer_id.to_string();
                let addr_strs: Vec<String> = addrs.iter().map(|a| a.to_string()).collect();

                // Persist each address for future reconnection.
                for addr in &addr_strs {
                    peers_store::add(addr);
                }

                // Don't double-list peers we're already connected to.
                if !app.connected_peers.iter().any(|(id, _)| id == &pid_str) {
                    match app.discovered_peers.iter_mut().find(|(id, _)| id == &pid_str) {
                        Some((_, existing_addrs)) => {
                            for a in &addr_strs {
                                if !existing_addrs.contains(a) {
                                    existing_addrs.push(a.clone());
                                }
                            }
                        }
                        None => {
                            app.discovered_peers.push((pid_str.clone(), addr_strs.clone()));
                        }
                    }
                }
                app.status.peers_discovered = app.discovered_peers.len();
                app.push_traffic(format!(
                    "Peer discovered: {}  ({})",
                    &pid_str[..pid_str.len().min(20)],
                    addr_strs.first().map(String::as_str).unwrap_or("-")
                ));

                // Auto-connect to every newly discovered peer.
                for addr_str in &addr_strs {
                    if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
                        let _ = cmd_tx.send(ControlCommand::Connect(addr));
                    }
                }
            }
            NodeEvent::TransactionReceived(tx) => {
                app.push_traffic(format!("Transaction received: {} ({:?})", tx.id, tx.kind));
            }
            NodeEvent::BlockReceived(block) => {
                app.push_traffic(format!(
                    "Block received: #{} ({} txs)",
                    block.index,
                    block.transactions.len()
                ));
            }
            NodeEvent::BlockFinalised { block_index } => {
                app.push_traffic(format!("Block finalised: #{block_index}"));
            }
            NodeEvent::ChainSynced { new_length } => {
                app.status.chain_length = new_length;
                app.push_traffic(format!("Chain synced — length {new_length}"));
            }
        },

        ControlEvent::Error(msg) => {
            app.push_traffic(format!("error: {msg}"));
            app.set_output(format!("error: {msg}"));
        }
    }
}

/// Resolve the path for the TUI's log file.
///
/// Uses `$XDG_DATA_HOME/spn/spn.log` when the env var is set, otherwise
/// falls back to `~/.local/share/spn/spn.log`.
fn spn_log_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("spn").join("spn.log")
}

/// Parse a `start-end` port range string (e.g. `"51025-51030"`).
fn parse_port_range(s: &str) -> Option<(u16, u16)> {
    let (start, end) = s.split_once('-')?;
    let start = start.trim().parse::<u16>().ok()?;
    let end = end.trim().parse::<u16>().ok()?;
    if start <= end { Some((start, end)) } else { None }
}

/// Convert a plain `ip:port` string to a `/ip4/<ip>/tcp/<port>` multiaddr string.
fn parse_ip_port(s: &str) -> Option<String> {
    let (ip, port) = s.rsplit_once(':')?;
    port.parse::<u16>().ok()?;
    Some(format!("/ip4/{ip}/tcp/{port}"))
}

/// Look through connected peers for one whose address list contains `addr_str`
/// (or its multiaddr equivalent).  Returns the parsed [`libp2p::PeerId`] if found.
fn find_peer_by_addr(
    connected_peers: &[(String, Vec<String>)],
    addr_str: &str,
) -> Option<libp2p::PeerId> {
    let alt = parse_ip_port(addr_str);

    for (pid_str, addrs) in connected_peers {
        let matched = addrs.iter().any(|a| {
            a == addr_str || alt.as_deref().map(|alt| a == alt).unwrap_or(false)
        });
        if matched {
            return pid_str.parse::<libp2p::PeerId>().ok();
        }
    }
    None
}
