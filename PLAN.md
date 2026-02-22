# Social Production Network

## Needs

- A library that implements the architecture
- A TUI to
  - Embeds node
  - Start/Stop/Restart as a node
  - See the traffic on the node
  - See the node's status
  - See nodes to connect to
  - See nodes currently connected
  - Disconnect from a known node
  - Connect to a known node
- A CLI to run node
- A binary to run node
  - Installation script to install node (support linux, mac, and windows)
  - Uninstall script to uninstall node (support linux, mac, and windows)
  - Systemd service for node

## Library

- Include API to choose discovery mode (default Kademlia DHT)
- Include API to choose a port to start on (defaults to 51025)
- Include API to choose a port range for discovery (defaults to all)
- Include API to run as gossip

## TUI

- binary is named spn

### Layout

- No borders on any elements
- Uses color scheme based on green

#### General

```ascii
+-----------------------------------+
| Social Production Node            |
+-----------------------------------+
| Content                           |
+-----------------------------------+
| Input                             |
+-----------------------------------+
```

### Commands

| Command | Description |
| /start | Starts the node |
| /stop | Stops the node |
| /restart | Restart the node |
| /traffic | See the node's traffic |
| /status | See the node's status |
| /discover <start port>-<end port> | Discover nodes to connect to |
| /connected | See the nodes that are currently connected |
| /disconnect <node id> or <ip>:<port> | Disconnect from a node |
| /connect <ip>:<port> | Connect a node |

### Operations

- When the tui starts, do a discovery and autoconnect to found peers
- When peers are added or connected to, store the peers for future connections
- I should be able to scroll through the prompt/input history and rerun commands
- Discover
  - Searches across the internet for peers
  - If no start or end port range is given, search on the port the node is running on
  - Periodically run discover to find new nodes

## Architecture

- P2P Network
  - Each app is a node on the network
  - A node server can be independent of running the app
  - A node can act as a gossip without downloading any assets
  - Syncing will be done using a blockchain
  - Peers will be pinged periodically to see if they are still alive
    - A peer will be disconnected if it not alive
  - Peer discovery will run at the beginning and periodically to ensure new peers are picked up
- Blockchain
  - Only transactions live in the blockchain
  - Transactions live in Merkle Trees as storage
  - Synching will always sync all of the blockchain
  - A transaction will be considered valid if the blockchain has been verified by a minimum of 3 nodes
- Transaction
  - A transaction can be any change event that has happened on the platform. For example:
    - A user was:
      - Registered
      - Edited
      - Unregistered
    - An Organization was:
      - Registered
      - Edited
      - Unregistered
    - A Project was:
      - Posted
      - Edited
      - Status changed
    - A Project update was:
      - Added
      - Edited
      - Deleted
    - Funding for a project was:
      - Created
      - Funded
      - Distributed
    - A Post was:
      - Created
      - Updated
      - Deleted
    - A Comment was added
    - An event was:
      - Added
      - Edited
      - Cancelled
    - RSVP changes
    - Any votes that happen
    - A node was added
    - A node was removed
- Resource usage is controllable
  - Assets can be synced at a granular level
    - Sync based on a time range
    - Sync based on file size or disk availability
    - Sync on demand
  - Content can be synced at a granular level
    - Sync based on a time range
    - Sync based on file size or disk availability
    - Sync on demand
- Verification
  - Each node approves a new block
  - After 3 nodes approve a block, the blockchain is considered valid
