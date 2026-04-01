# Social Production Network

A decentralized p2p network for social production

## Technology

- Rust 2024 edition
- GRPC

## Key components

- A P2P network for discovery and connecting to peers
- A database that is the record of the system
- A blockchain for syncing databases
- A reputation system to filter peer nodes as a source of truth

## P2P network

The P2P network is used for discovery of peers.

Each peer node will:

- Be able to find other peer nodes:
  - Local via mDNS
  - Remotely using Kademia
- Add discovered peer nodes to a list of nodes previously discovered peer nodes
- Be able to connect directly with discovered peer nodes to:
  - Sync with their latest blockchain
  - Sync content
  - Upload new blockchain blocks
  - Discover new peers faster
- Run in different modes:
  - Full
  - Light
  - Gossip
- Contain a user

### Peer node model

A peer node has various components to make it work.

#### The node_id

When a node is started, if there are no node keys, two keys are started:

- node_key: the private key for a node
- node_id: the public key for a node

These are generated via ECDH and stored in the `~/.socplan/node` directory as:

- `key`: known as **node_key** in the code
- `id`: known as **node_id** in the code

#### The user_id

A user_id is applicable when a user has been created or imported. All the ECDH information for a user is stored in the `~/.socplan/user` directory as:

- `key`: known as **user_key** in the code
- `id`: known as **user_id** in the code

#### Discovered nodes

Discovered peer nodes are stored in the `~/.socplan/discovered` directory.

The process follows like this.

When a peer node is discovered:

1. A `~/.socplan/discovered/<node_id>` directory is created
1. An `~/.socplan/discovered/<node_id>/known_address` file is created with the **address** of the node as the content
1. A `~/.socplan/discovered/<node_id>/last_seen` file is created with the current timestamp in unix seconds

We use the **known_address** later to directly connect with the node for processes like syncing and sub peer discovery.

The **last_seen** file will help us filter out peer nodes that have not been seen in a while, to see the most recent, etc.
