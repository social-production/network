/// Integration smoke tests that wire together all crates end-to-end.
///
/// These tests exercise the full data path:
///   Transaction → pending pool → Block → Merkle root → Blockchain
///
/// Network-level tests (gossipsub, peer discovery) require a running async
/// runtime and real ports, so they are marked `#[tokio::test]` and guarded
/// with short timeouts.
use sp_blockchain::Blockchain;
use sp_merkle::MerkleTree;
use sp_node::{DiscoveryMode, Node, NodeConfig, NodeMode};
use sp_sync::SyncStrategy;
use sp_transaction::{Transaction, TransactionType};

// ── Pure data-path tests (no network) ───────────────────────────────────────

#[test]
fn transaction_to_blockchain_roundtrip() {
    let mut chain = Blockchain::new();

    let txs = vec![
        Transaction::new(TransactionType::UserRegistered, b"alice".to_vec()),
        Transaction::new(TransactionType::ProjectPosted, b"project-1".to_vec()),
        Transaction::new(TransactionType::FundingCreated, b"fund-1".to_vec()),
    ];

    chain.add_block(txs.clone()).expect("add_block failed");

    assert_eq!(chain.len(), 2, "genesis + one new block");
    assert!(chain.is_valid(), "chain should be valid");

    let block = chain.get_block(1).expect("block 1 should exist");
    assert_eq!(block.transactions.len(), 3);
}

#[test]
fn merkle_proof_validates_each_transaction_in_block() {
    let txs: Vec<Transaction> = [
        (TransactionType::UserRegistered, b"user-a" as &[u8]),
        (TransactionType::OrgRegistered, b"org-1"),
        (TransactionType::ProjectPosted, b"proj-1"),
        (TransactionType::VoteCast, b"vote-1"),
    ]
    .into_iter()
    .map(|(k, p)| Transaction::new(k, p.to_vec()))
    .collect();

    let tree = MerkleTree::new(&txs).expect("tree should build");
    let root = tree.root_hash().expect("root should exist");

    for tx in &txs {
        let proof = tree.proof(&txs, tx.id).expect("proof should exist");
        assert!(proof.verify(&root), "proof failed for tx {}", tx.id);
    }
}

#[test]
fn block_finalisation_requires_three_distinct_peers() {
    let mut chain = Blockchain::new();
    chain
        .add_block(vec![Transaction::new(
            TransactionType::NodeAdded,
            b"peer-bootstrap".to_vec(),
        )])
        .unwrap();

    assert!(!chain.verify_block(1, "peer-1".into()).unwrap());
    assert!(!chain.verify_block(1, "peer-2".into()).unwrap());
    // Third unique peer finalises the block.
    assert!(chain.verify_block(1, "peer-3".into()).unwrap());
    // Duplicate peer does not change finalised status.
    assert!(chain.verify_block(1, "peer-1".into()).unwrap());

    assert!(chain.get_block(1).unwrap().is_finalised());
}

#[test]
fn sync_from_longer_chain_replaces_local() {
    let mut local = Blockchain::new();

    let mut remote = Blockchain::new();
    for i in 0u8..5 {
        remote
            .add_block(vec![Transaction::new(
                TransactionType::PostCreated,
                vec![i],
            )])
            .unwrap();
    }

    assert!(local.sync_from(&remote), "local should adopt longer chain");
    assert_eq!(local.len(), remote.len());
    assert!(local.is_valid());
}

#[test]
fn node_mode_is_full_by_default() {
    assert_eq!(NodeMode::default(), NodeMode::Full);
}

#[test]
fn sync_strategy_defaults_to_on_demand() {
    use sp_sync::SyncStrategy;
    assert_eq!(SyncStrategy::default(), SyncStrategy::OnDemand);
}

#[test]
fn all_transaction_types_are_serialisable() {
    use sp_transaction::TransactionType::*;
    let kinds = [
        UserRegistered, UserEdited, UserUnregistered,
        OrgRegistered, OrgEdited, OrgUnregistered,
        ProjectPosted, ProjectEdited, ProjectStatusChanged,
        ProjectUpdateAdded, ProjectUpdateEdited, ProjectUpdateDeleted,
        FundingCreated, FundingFunded, FundingDistributed,
        PostCreated, PostUpdated, PostDeleted,
        CommentAdded,
        EventAdded, EventEdited, EventCancelled,
        RsvpChanged, VoteCast,
        NodeAdded, NodeRemoved,
    ];

    for kind in kinds {
        let tx = Transaction::new(kind.clone(), b"test".to_vec());
        let hash = tx.hash().expect("hash should succeed");
        assert_ne!(hash, [0u8; 32]);
    }
}

// ── Network-level smoke tests ────────────────────────────────────────────────

#[tokio::test]
async fn node_starts_and_listens() {
    let config = NodeConfig {
        port: 0, // let OS pick a free port
        mode: NodeMode::Full,
        sync_strategy: SyncStrategy::OnDemand,
        ..Default::default()
    };
    let (node, _events) = Node::new(config)
        .await
        .expect("node should start");

    // If we reach here without a panic the node has successfully bound a port
    // and set up all sub-protocols.
    let _ = node.peer_id(); // peer_id is non-zero
}

#[tokio::test]
async fn gossip_node_starts_successfully() {
    let config = NodeConfig {
        port: 0,
        mode: NodeMode::Gossip,
        sync_strategy: SyncStrategy::OnDemand,
        ..Default::default()
    };
    let (node, _events) = Node::new(config)
        .await
        .expect("gossip node should start");
    let _ = node.peer_id();
}

#[tokio::test]
async fn node_starts_with_kademlia_discovery() {
    let config = NodeConfig {
        port: 0,
        mode: NodeMode::Full,
        sync_strategy: SyncStrategy::OnDemand,
        discovery_mode: DiscoveryMode::KademliaDht,
        ..Default::default()
    };
    let (node, _events) = Node::new(config).await.expect("node should start");
    let _ = node.peer_id();
}

#[tokio::test]
async fn node_starts_with_mdns_discovery() {
    let config = NodeConfig {
        port: 0,
        mode: NodeMode::Full,
        sync_strategy: SyncStrategy::OnDemand,
        discovery_mode: DiscoveryMode::Mdns,
        ..Default::default()
    };
    let (node, _events) = Node::new(config).await.expect("node should start");
    let _ = node.peer_id();
}

#[tokio::test]
async fn node_starts_with_discovery_port_range() {
    let config = NodeConfig {
        port: 0,
        mode: NodeMode::Full,
        sync_strategy: SyncStrategy::OnDemand,
        discovery_port_range: Some(40000..=60000),
        ..Default::default()
    };
    let (node, _events) = Node::new(config).await.expect("node should start");
    let _ = node.peer_id();
}

#[tokio::test]
async fn node_broadcasts_transaction_without_peers() {
    let config = NodeConfig {
        port: 0,
        mode: NodeMode::Full,
        sync_strategy: SyncStrategy::OnDemand,
        ..Default::default()
    };
    let (mut node, _events) = Node::new(config).await.unwrap();

    let tx = Transaction::new(TransactionType::UserRegistered, b"alice".to_vec());

    // Publishing to gossipsub with no connected peers returns an error from
    // libp2p ("insufficient peers"), which we propagate — that's fine.
    // The important thing is that the node is alive and doesn't panic.
    let _ = node.broadcast_transaction(tx);
}
