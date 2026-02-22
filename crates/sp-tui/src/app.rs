use std::collections::VecDeque;

/// Maximum number of traffic entries kept in memory.
const MAX_TRAFFIC: usize = 500;

/// High-level state of the node lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeState {
    Stopped,
    Starting,
    Running,
    Restarting,
}

impl NodeState {
    pub fn label(&self) -> &str {
        match self {
            NodeState::Stopped => "stopped",
            NodeState::Starting => "starting…",
            NodeState::Running => "running",
            NodeState::Restarting => "restarting…",
        }
    }
}

/// Which panel is shown in the content area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentView {
    /// Live event log (default).
    Traffic,
    /// Node status key-value snapshot.
    Status,
    /// Peers discovered but not yet connected.
    Discovered,
    /// Currently connected peers.
    Connected,
}

/// A timestamped traffic event.
#[derive(Debug, Clone)]
pub struct TrafficEntry {
    pub timestamp: String,
    pub message: String,
}

/// Snapshot of node statistics shown in the status view.
#[derive(Debug, Clone, Default)]
pub struct NodeStatus {
    pub peer_id: String,
    pub listen_addr: String,
    pub peers_connected: usize,
    pub peers_discovered: usize,
    pub chain_length: usize,
    pub pending_txs: usize,
    pub mode: String,
    pub sync_strategy: String,
    pub discovery_mode: String,
    pub port: u16,
}

/// Maximum entries kept in the command history shown in the input panel.
const MAX_HISTORY: usize = 200;

/// The complete TUI state.
pub struct App {
    pub node_state: NodeState,
    pub status: NodeStatus,
    pub traffic: VecDeque<TrafficEntry>,
    pub traffic_scroll: usize,
    pub view: ContentView,
    /// Text the user is currently typing.
    pub input: String,
    /// History of commands executed this session (most recent last).
    pub command_history: VecDeque<String>,
    /// Position within `command_history` during ↑/↓ navigation.
    /// `None` means not navigating (user is editing fresh input).
    pub history_cursor: Option<usize>,
    /// Snapshot of `input` saved the moment history navigation begins,
    /// restored when the user scrolls back past the most-recent command.
    pub input_snapshot: String,
    /// Optional one-line feedback message shown below the input (error / info).
    pub command_output: Option<String>,
    /// Discovered but not yet connected peers: (peer_id_str, addrs).
    pub discovered_peers: Vec<(String, Vec<String>)>,
    /// Currently connected peers: (peer_id_str, addrs).
    pub connected_peers: Vec<(String, Vec<String>)>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            node_state: NodeState::Stopped,
            status: NodeStatus {
                mode: "Full".into(),
                sync_strategy: "OnDemand".into(),
                discovery_mode: "KademliaDht".into(),
                port: 51025,
                ..Default::default()
            },
            traffic: VecDeque::new(),
            traffic_scroll: 0,
            view: ContentView::Traffic,
            input: String::new(),
            command_history: VecDeque::new(),
            history_cursor: None,
            input_snapshot: String::new(),
            command_output: None,
            discovered_peers: Vec::new(),
            connected_peers: Vec::new(),
            should_quit: false,
        }
    }

    /// Record a command in the history log.
    pub fn push_history(&mut self, cmd: impl Into<String>) {
        self.command_history.push_back(cmd.into());
        if self.command_history.len() > MAX_HISTORY {
            self.command_history.pop_front();
        }
    }

    /// Move backward through history (↑).  Saves the live input on first call.
    pub fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let max_idx = self.command_history.len() - 1;
        match self.history_cursor {
            None => {
                self.input_snapshot = self.input.clone();
                self.history_cursor = Some(max_idx);
            }
            Some(idx) if idx > 0 => {
                self.history_cursor = Some(idx - 1);
            }
            _ => return, // already at oldest
        }
        if let Some(idx) = self.history_cursor {
            if let Some(cmd) = self.command_history.get(idx) {
                self.input = cmd.clone();
            }
        }
    }

    /// Move forward through history (↓).  Restores live input when past the newest.
    pub fn history_next(&mut self) {
        match self.history_cursor {
            None => {}
            Some(idx) => {
                let max_idx = self.command_history.len().saturating_sub(1);
                if idx < max_idx {
                    let new_idx = idx + 1;
                    self.history_cursor = Some(new_idx);
                    if let Some(cmd) = self.command_history.get(new_idx) {
                        self.input = cmd.clone();
                    }
                } else {
                    self.history_cursor = None;
                    self.input = self.input_snapshot.clone();
                }
            }
        }
    }

    /// Reset history navigation state (call on Enter or any typed character).
    pub fn reset_history_nav(&mut self) {
        self.history_cursor = None;
        self.input_snapshot = String::new();
    }

    /// Push a timestamped entry into the traffic log.
    pub fn push_traffic(&mut self, message: impl Into<String>) {
        use chrono::Local;
        let entry = TrafficEntry {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            message: message.into(),
        };
        self.traffic.push_back(entry);
        if self.traffic.len() > MAX_TRAFFIC {
            self.traffic.pop_front();
        }
        self.traffic_scroll = self.traffic.len().saturating_sub(1);
    }

    pub fn scroll_traffic_up(&mut self) {
        self.traffic_scroll = self.traffic_scroll.saturating_sub(1);
    }

    pub fn scroll_traffic_down(&mut self) {
        let max = self.traffic.len().saturating_sub(1);
        if self.traffic_scroll < max {
            self.traffic_scroll += 1;
        }
    }

    pub fn set_output(&mut self, msg: impl Into<String>) {
        self.command_output = Some(msg.into());
    }

    pub fn clear_output(&mut self) {
        self.command_output = None;
    }
}
