use ratatui::{
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, ContentView, NodeState};

// ── Green-based colour palette ────────────────────────────────────────────────
const PRIMARY: Color = Color::Green;
const BRIGHT: Color = Color::LightGreen;
const DIM: Color = Color::DarkGray;
const WARN: Color = Color::Yellow;
const DANGER: Color = Color::Red;
const MUTED: Color = Color::Gray;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // ── Three-row outer layout ────────────────────────────────────────────────
    //   Row 0: header  (1 line)
    //   Row 1: content (fills remaining space)
    //   Row 2: input   (3 lines: hint + feedback + prompt)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, app, rows[0]);
    draw_content_panel(frame, app, rows[1]);
    draw_input_area(frame, app, rows[2]);
}

// ── Header (row 0) ────────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let state_color = match app.node_state {
        NodeState::Running => PRIMARY,
        NodeState::Stopped => DANGER,
        NodeState::Starting | NodeState::Restarting => WARN,
    };

    let line = Line::from(vec![
        Span::styled(
            "Social Production Node",
            Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", app.node_state.label()),
            Style::default().fg(state_color),
        ),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

// ── Content panel (row 1) ─────────────────────────────────────────────────────

fn draw_content_panel(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let inner = area.inner(Margin { horizontal: 1, vertical: 0 });

    match &app.view {
        ContentView::Traffic => draw_traffic(frame, app, inner),
        ContentView::Status => draw_status(frame, app, inner),
        ContentView::Discovered => {
            draw_peer_list(frame, &app.discovered_peers, "discovered peers", inner);
        }
        ContentView::Connected => {
            draw_peer_list(frame, &app.connected_peers, "connected peers", inner);
        }
    }
}

// ── Traffic view ──────────────────────────────────────────────────────────────

fn draw_traffic(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let total = app.traffic.len();

    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("traffic  ({total} events)"),
            Style::default().fg(DIM),
        )),
        split[0],
    );

    let items: Vec<ListItem> = app
        .traffic
        .iter()
        .map(|entry| {
            let ts = Span::styled(
                format!("{} ", entry.timestamp),
                Style::default().fg(DIM),
            );
            let msg = Span::styled(entry.message.clone(), traffic_style(&entry.message));
            ListItem::new(Line::from(vec![ts, msg]))
        })
        .collect();

    let mut list_state = ListState::default();
    if total > 0 {
        list_state.select(Some(app.traffic_scroll));
    }

    let list = List::new(items)
        .highlight_style(Style::default().fg(BRIGHT).add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(list, split[1], &mut list_state);

    if total > 0 {
        let mut sb_state = ScrollbarState::new(total).position(app.traffic_scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            split[1],
            &mut sb_state,
        );
    }
}

// ── Status view ───────────────────────────────────────────────────────────────

fn draw_status(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let s = &app.status;

    let peer_id_str = truncate(&s.peer_id, 48);
    let listen_str = truncate(&s.listen_addr, 48);
    let port_str = s.port.to_string();
    let peers_connected_str = s.peers_connected.to_string();
    let peers_discovered_str = s.peers_discovered.to_string();
    let chain_str = s.chain_length.to_string();
    let pending_str = s.pending_txs.to_string();

    let state_color = match app.node_state {
        NodeState::Running => PRIMARY,
        NodeState::Stopped => DANGER,
        _ => WARN,
    };

    let rows: Vec<Line> = vec![
        kv_row("node state", app.node_state.label(), state_color),
        kv_row("peer id", &peer_id_str, MUTED),
        kv_row("listen addr", &listen_str, MUTED),
        kv_row("port", &port_str, PRIMARY),
        kv_row("mode", &s.mode, PRIMARY),
        kv_row("discovery", &s.discovery_mode, PRIMARY),
        kv_row("sync", &s.sync_strategy, PRIMARY),
        kv_row("peers connected", &peers_connected_str, BRIGHT),
        kv_row("peers discovered", &peers_discovered_str, BRIGHT),
        kv_row("chain length", &chain_str, BRIGHT),
        kv_row("pending txs", &pending_str, BRIGHT),
    ];

    let items: Vec<ListItem> = rows.into_iter().map(ListItem::new).collect();
    frame.render_widget(List::new(items), area);
}

// ── Peer list view (discovered / connected) ───────────────────────────────────

fn draw_peer_list(
    frame: &mut Frame,
    peers: &[(String, Vec<String>)],
    title: &str,
    area: ratatui::layout::Rect,
) {
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{title}  ({} peers)", peers.len()),
            Style::default().fg(DIM),
        )),
        split[0],
    );

    let items: Vec<ListItem> = if peers.is_empty() {
        vec![ListItem::new(Span::styled(
            "  none",
            Style::default().fg(DIM),
        ))]
    } else {
        peers
            .iter()
            .flat_map(|(pid, addrs)| {
                let id_item = ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        truncate(pid, 56),
                        Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
                    ),
                ]));
                let addr_items: Vec<ListItem> = addrs
                    .iter()
                    .map(|a| {
                        ListItem::new(Span::styled(
                            format!("    {}", truncate(a, 54)),
                            Style::default().fg(MUTED),
                        ))
                    })
                    .collect();
                std::iter::once(id_item).chain(addr_items)
            })
            .collect()
    };

    frame.render_widget(List::new(items), split[1]);
}

// ── Input area (row 2) ────────────────────────────────────────────────────────
//
//   Line 0: key hints (dim)
//   Line 1: feedback / error message
//   Line 2: > prompt

fn draw_input_area(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // hints
            Constraint::Length(1), // feedback
            Constraint::Length(1), // prompt
        ])
        .split(area);

    // Key hints.
    frame.render_widget(
        Paragraph::new(Span::styled(
            "↑/↓ history · PgUp/PgDn scroll · /help for commands · Ctrl-C quit",
            Style::default().fg(DIM),
        )),
        rows[0],
    );

    // Feedback / error line.
    let feedback_text = app.command_output.as_deref().unwrap_or("");
    let feedback_style = if feedback_text.starts_with("error") || feedback_text.starts_with("Error") {
        Style::default().fg(DANGER)
    } else if !feedback_text.is_empty() {
        Style::default().fg(WARN)
    } else {
        Style::default().fg(DIM)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(feedback_text.to_string(), feedback_style)),
        rows[1],
    );

    // Prompt line.
    let prompt = Line::from(vec![
        Span::styled("> ", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}_", app.input), Style::default().fg(BRIGHT)),
    ]);
    frame.render_widget(Paragraph::new(prompt), rows[2]);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn kv_row<'a>(label: &'a str, value: &'a str, value_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", label), Style::default().fg(DIM)),
        Span::styled(value.to_owned(), Style::default().fg(value_color)),
    ])
}

fn traffic_style(msg: &str) -> Style {
    if msg.contains("finalised") || msg.contains("synced") {
        Style::default().fg(BRIGHT)
    } else if msg.contains("Transaction") || msg.contains("transaction") {
        Style::default().fg(PRIMARY)
    } else if msg.contains("Block") || msg.contains("block") {
        Style::default().fg(Color::Cyan)
    } else if msg.contains("connected") && !msg.contains("dis") || msg.contains("started") {
        Style::default().fg(PRIMARY)
    } else if msg.contains("disconnected") || msg.contains("stopped") || msg.contains("error") {
        Style::default().fg(DANGER)
    } else if msg.contains("discovered") || msg.contains("Listening") {
        Style::default().fg(MUTED)
    } else {
        Style::default().fg(MUTED)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
