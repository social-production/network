# Social Production Network

A peer-to-peer network library, node daemon, and terminal UI for the Social Production platform.  Nodes gossip transactions, form blocks, verify the chain, and sync state — all without a central server.

---

## Architecture overview

```
┌─────────────────────────────────────────────────────┐
│  spn  (TUI)          sp-node  (CLI daemon)           │
│  embeds node         standalone binary               │
└────────────┬─────────────────────┬──────────────────┘
             │                     │
             ▼                     ▼
┌────────────────────────────────────────────────────┐
│                   sp-node  (library)               │
│  libp2p swarm: gossipsub · Kademlia · mDNS · ping  │
│  NodeConfig API · peer management · block sync     │
└──────┬──────────┬───────────────────────────────────┘
       │          │
       ▼          ▼
 sp-blockchain  sp-sync
 sp-merkle      sp-transaction
```

### Crates

| Crate | Description |
|-------|-------------|
| `sp-node` | P2P node library + `sp-node` CLI binary |
| `sp-tui` | Terminal UI (`spn` binary) |
| `sp-blockchain` | Append-only blockchain with 3-node verification |
| `sp-merkle` | Merkle tree storage for transactions |
| `sp-transaction` | Transaction types and hashing |
| `sp-sync` | Pluggable sync strategies (on-demand, time-range, size-limit) |

### P2P layer

Built on [libp2p](https://libp2p.io/) with:

- **Gossipsub** — epidemic broadcast of transactions, blocks, and verifications
- **Kademlia DHT** — internet-wide peer discovery and routing
- **mDNS** — zero-config local/LAN peer discovery (default: both enabled)
- **Ping** — periodic keepalive; unresponsive peers are disconnected automatically
- **Identify** — peers exchange listen addresses; used to populate Kademlia routing table

### Blockchain

- Only transactions live in the chain
- Transactions are stored in Merkle trees per block
- A block is finalised after **3 distinct nodes** verify it
- Sync always replaces the local chain with the longest valid remote chain

---

## Requirements

- [Rust](https://rustup.rs/) 1.75+ (edition 2021)
- `cargo`
- For the systemd service: Linux with systemd
- For the launchd service: macOS 10.15+
- For the Windows service: Windows 10/11, PowerShell 5.1+

---

## Quick start

### Build everything

```bash
cargo build --release
```

### Run the TUI (recommended)

```bash
cargo run -p sp-tui --bin spn
# or after installation:
spn
```

The TUI auto-starts the embedded node on launch, discovers nearby peers via mDNS, and reconnects to any previously known peers from `~/.config/spn/peers.json`.

### Run the CLI daemon

```bash
cargo run -p sp-node -- --help

# Examples:
sp-node                                  # full node, port 51025, 60 s discovery
sp-node --port 51026                     # custom port
sp-node --mode gossip                    # relay-only, no block storage
sp-node --discovery both                 # Kademlia + mDNS
sp-node --discovery-interval 30          # re-discover every 30 s
sp-node --quiet                          # suppress stderr output
```

---

## TUI reference

Launch with `spn`.  The layout is:

```
┌──────────────────────────────────┐
│ Social Production Node  [state]  │  ← header
├──────────────────────────────────┤
│                                  │
│  content view                    │  ← traffic / status / peers
│                                  │
├──────────────────────────────────┤
│ ↑/↓ history · PgUp/PgDn scroll  │  ← hints
│ feedback line                    │
│ > _                              │  ← input prompt
└──────────────────────────────────┘
```

### Commands

| Command | Description |
|---------|-------------|
| `/start` | Start the embedded node |
| `/stop` | Stop the node |
| `/restart` | Restart the node |
| `/traffic` | Show the live event log (default view) |
| `/status` | Show node statistics |
| `/discover [start-end]` | Scan for peers; optional port range e.g. `/discover 51025-51030` |
| `/connected` | Show currently connected peers |
| `/disconnect <id\|ip:port>` | Disconnect a peer by peer-id or address |
| `/connect <ip:port>` | Connect to a specific peer |
| `/help` | Print all commands to the traffic log |
| `/quit` | Stop the node and exit |

### Key bindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Scroll through command history |
| `PgUp` / `PgDn` | Scroll the content view |
| `Ctrl-C` | Quit immediately |

### Logs

TUI logs are written to `$XDG_DATA_HOME/spn/spn.log` (default `~/.local/share/spn/spn.log`) so they never bleed onto the screen.

---

## CLI reference (`sp-node`)

```
sp-node [OPTIONS]

Options:
  -p, --port <PORT>                        Listen port [default: 51025] [env: SPN_PORT]
  -m, --mode <MODE>                        full | gossip [default: full] [env: SPN_MODE]
  -d, --discovery <DISCOVERY>              kademlia | mdns | both [default: kademlia] [env: SPN_DISCOVERY]
  -s, --sync <SYNC>                        on-demand | all [default: on-demand] [env: SPN_SYNC]
      --discovery-port-min <MIN>           Filter discovered addresses to ports >= MIN [env: SPN_DISCOVERY_PORT_MIN]
      --discovery-port-max <MAX>           Filter discovered addresses to ports <= MAX [env: SPN_DISCOVERY_PORT_MAX]
      --discovery-interval <SECS>          Re-discover every N seconds [default: 60] [env: SPN_DISCOVERY_INTERVAL]
  -q, --quiet                              Suppress stderr output [env: SPN_QUIET]
```

The daemon auto-discovers peers on startup and then repeats discovery on the configured interval.

---

## Library usage

```toml
[dependencies]
sp-node = { path = "crates/sp-node" }
```

```rust
use sp_node::{Node, NodeConfig, DiscoveryMode};

let config = NodeConfig {
    port: 51025,
    discovery_mode: DiscoveryMode::Both,
    ..NodeConfig::default()
};

let (mut node, mut events) = Node::new(config).await?;

tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        println!("{event:?}");
    }
});

// Run with periodic discovery every 60 s:
node.run_with_periodic_discovery(std::time::Duration::from_secs(60)).await;
```

### `NodeConfig` API

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `51025` | TCP listen port |
| `discovery_mode` | `DiscoveryMode` | `Both` | `KademliaDht`, `Mdns`, or `Both` |
| `discovery_port_range` | `Option<RangeInclusive<u16>>` | `None` | Filter discovered peer addresses by port |
| `mode` | `NodeMode` | `Full` | `Full` (validates) or `Gossip` (relay-only) |
| `sync_strategy` | `SyncStrategy` | `OnDemand` | When to sync blocks from peers |
| `quiet` | `bool` | `false` | Signal to the host binary to suppress logging |

---

## Installation

### Linux

```bash
./scripts/install.sh              # installs to /usr/local/bin
sudo ./scripts/install.sh         # also installs systemd service

# Enable the service:
sudo systemctl enable --now sp-node

# Uninstall:
./scripts/uninstall.sh
```

### macOS

```bash
./scripts/install.sh              # installs to /usr/local/bin + launchd agent

# Load the launchd agent (auto-started at login):
launchctl load ~/Library/LaunchAgents/com.socialproduction.spnode.plist

# Uninstall:
./scripts/uninstall.sh
```

### Windows

```powershell
# From repo root (standard user — no service registration):
powershell -ExecutionPolicy Bypass -File scripts\install.ps1

# As Administrator — also registers a Windows Service:
powershell -ExecutionPolicy Bypass -File scripts\install.ps1

# Start the service:
Start-Service sp-node

# Uninstall:
powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1
```

The binary is installed to `%LOCALAPPDATA%\Programs\spn\bin\sp-node.exe` and added to the user `PATH`.

### Custom install prefix

```bash
./scripts/install.sh --prefix ~/.local
./scripts/uninstall.sh --prefix ~/.local
```

```powershell
.\scripts\install.ps1 -Prefix "C:\Tools\spn"
```

---

## Development

```bash
# Run all tests
cargo test --workspace

# Run only the node integration tests
cargo test -p sp-node

# Check formatting
cargo fmt --check

# Lint
cargo clippy --workspace
```

Logs from tests are suppressed by default.  Set `RUST_LOG=sp_node=debug` to see them.

---

## Peer storage

The TUI persists known peer multiaddrs to `$XDG_CONFIG_HOME/spn/peers.json` (default `~/.config/spn/peers.json`).  On the next launch the node auto-dials every stored address.  Addresses are added automatically whenever a peer is discovered or connected.

---

## License

See [LICENSE](LICENSE) (to be added).
