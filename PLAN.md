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
  - Download content
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
1. A `~/.socplan/discovered/<node_id>/last_seen` file is created as a list of timestamps, with the current timestamp in unix seconds

We use the **known_address** later to directly connect with the node for processes like syncing and sub peer discovery.

The **last_seen** file will help us filter out peer nodes that have not been seen in a while, to see the most recent, etc.

## Database

Content in Social Production will be content addressable and stored on the local file system. This pertains to everything that is considered the database.

The way that this works is that we have `~/.socplan/database` as the base directory for content. All content is stored as:

- `~/.socplan/database/<content_id>/type`: the type of content (asset, project, thread, comment, etc.)
- `~/.socplan/database/<content_id>/meta`: information such as the title, user, type, created at, and so on
- `~/.socplan/database/<content_id>/body`: the body of the content
- `~/.socplan/database/<content_id>/tags`: a list of tags for the content
- `~/.socplan/database/<content_id>/channels`: a list of channels that the content is in
- `~/.socplan/database/<content_id>/votes`: a list of votes by user
- `~/.socplan/database/<content_id>/comments`: a list of content IDs that are comments
- `~/.socplan/database/<content_id>/updates`: a list of content IDs that are updates
- `~/.socplan/database/<content_id>/members`: a list of user IDs that are members
- `~/.socplan/database/<content_id>/events`: a list of content IDs that are events
- `~/.socplan/database/<content_id>/fund`: a structure that is used to fund a project type
- `~/.socplan/database/<content_id>/transactions`: a list of transactions to the fund (both receiving and spending)

Below are the types of content we expect and how the structure is affected.

### An asset

An asset is the most base type of content. It is used to store media such as an image, movie, etc.

The structure of an asset is like:

- has a
  - type
  - body

#### type

```json
"asset"
```

#### meta

```json
{
  "mime-type": "<mime-type of asset>"
}
```

#### body

```plaintext
<the body of the asset, base64 encoded>
```

#### tags

```json
[]
```

#### channels

```json
[]
```

#### votes

```json
[]
```

#### comments

Empty

#### updates

```json
[]
```

#### members

```json
[]
```

#### events

```json
[]
```

#### fund

```json
{}
```

#### transactions

```json
[]
```

##### A thread

A thread is a type of content that:

- has many
  - tags
  - votes
  - comments
- has a
  - type
  - title
  - owner
  - created at timestamp in unix seconds with a timezone
  - updated at timestamp in unix seconds with a timezone
  - body

As such, not all the files in the content structure will be filled. Here is what it should look like.

#### type

```json
"thread"
```

#### meta

```json
{
  "created_at": "<timestamp>",
  "updated_at": "<timestamp>",
  "title": "<title of post>",
  "owner_id": "<public key of user>"
}
```

#### body

```plaintext
<the body of the thread as markdown>
```

#### tags

```json
["tag1", "tag2", "tag3", "etc"]
```

#### channels

```json
["channel1", "channel2", "etc"]
```

#### votes

```json
[
  {
    "user_id": "<public key of user>",
    "state": "up"
  }
]
```

#### comments

```json
["<uuid of comment>", "<uuid of comment>", "<uuid of comment>"]
```

#### updates

```json
[]
```

#### members

```json
[]
```

#### events

```json
[]
```

#### fund

```json
{}
```

#### transactions

```json
[]
```

##### A comment

A comment is a type of content that:

- has many
  - votes
  - comments
- has a
  - type
  - owner
  - parent
  - created at timestamp in unix seconds with a timezone
  - updated at timestamp in unix seconds with a timezone
  - body

As such, not all the files in the content structure will be filled. Here is what it should look like.

#### type

```json
"comment"
```

#### meta

```json
{
  "created_at": "<timestamp>",
  "updated_at": "<timestamp>",
  "title": "<title of post>",
  "owner_id": "<public key of user>",
  "parent_id": "<uuid of parent content>"
}
```

#### body

```plaintext
<the body of the comment as markdown>
```

#### tags

```json
[]
```

#### channels

```json
[]
```

#### votes

```json
[
  {
    "user_id": "<public key of user>",
    "state": "up"
  }
]
```

#### comments

```json
["<uuid of comment>", "<uuid of comment>", "<uuid of comment>"]
```

#### updates

```json
[]
```

#### members

```json
[]
```

#### events

```json
[]
```

#### fund

```json
{}
```

#### transactions

```json
[]
```

##### A project

A comment is a type of content that:

- has many
  - votes
  - tags
  - channels
  - comments
  - members
  - events
  - updates
- has a
  - type
  - owner
  - fund
  - status
  - created at timestamp in unix seconds with a timezone
  - updated at timestamp in unix seconds with a timezone
  - body

As such, not all the files in the content structure will be filled. Here is what it should look like.

#### type

```json
"project"
```

#### meta

```json
{
  "created_at": "<timestamp>",
  "updated_at": "<timestamp>",
  "title": "<title of project>",
  "owner_id": "<public key of user>",
  "status": "IN_PROGRESS"
}
```

#### body

```plaintext
<the body of the comment as markdown>
```

#### tags

```json
["tag1", "tag2"]
```

#### channels

```json
["channel1", "channel2"]
```

#### votes

```json
[
  {
    "user_id": "<public key of user>",
    "state": "up"
  }
]
```

#### comments

```json
["<uuid of comment>", "<uuid of comment>", "<uuid of comment>"]
```

#### updates

```json
[
  {
    "status": "PROPOSED",
    "created_at": "<timestamp>",
    "owner_id": "<public key of user>"
  }
]
```

#### members

```json
["<public key of user>", "<public key of user>"]
```

#### events

```json
[
  {
    "title": "<project title>",
    "content": "<markdown content>",
    "date": "<timestamp>",
    "location": "<location>",
    "rsvps": [
      {
        "user_id": "<public key of user>",
        "response": "yes"
      }
    ]
  }
]
```

#### fund

```json
{
  "title": "<title of the funds>",
  "amount": 12345.67,
  "currency": "USD",
  "committed": [
    {
      "user_id": "<public key of user>",
      "amount": 123.45
    }
  ]
}
```

#### transactions

```json
[
  {
    "user_id": "<public key of user>",
    "amount": 123.45,
    "currency": "USD",
    "meta": {}
  }
]
```

## Blockchain

A blockchain is necessary as a signal for changes and a record to keep the change history idempotent across the network.

There are some key features which must be encoroporated into a network node

- Storage: where the blockchain components are stored
- Modification: adding new blocks to the blockchain
- Syncronization: syncing the blockchain with other peers
- Verification: verifying incoming blocks
- Resolution: making determinations on whether a blockchain is valid or not and handling forks

### Storage

Like many of the node components, the blockchain is stored on the file system at `~/.socplan/blockchain`. Here are the components:

- `~/.socplan/blockchain/transactions`: where transactions are stored before being gathered into blocks
- `~/.socplan/blockchain/blocks`: where blocks are stored

#### Transactions

Transactions are considered emphemeral in that, they are only there waiting for a future block to be created. When a new block is created, they are transferred to the block and cleared out of the general storage area.

### Modification

### Syncronization

### Verification

### Resolution
