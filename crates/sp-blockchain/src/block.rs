use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sp_merkle::MerkleTree;
use sp_transaction::Transaction;

use crate::{BlockchainError, MIN_VERIFICATIONS};

/// A single block in the Social Production blockchain.
///
/// Transactions are stored directly in the block and their Merkle root is
/// committed in `merkle_root`.  A block is only *finalised* once at least
/// [`MIN_VERIFICATIONS`] distinct peer IDs have been recorded in
/// `verifications`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Sequential block height (genesis = 0).
    pub index: u64,

    /// SHA-256 hash of the previous block.  All-zero for the genesis block.
    pub prev_hash: [u8; 32],

    /// Merkle root of `transactions`.
    pub merkle_root: [u8; 32],

    /// All transactions bundled in this block.
    pub transactions: Vec<Transaction>,

    /// Unix timestamp (seconds) when this block was created.
    pub timestamp: i64,

    /// Simple proof-of-work nonce (reserved for future use; currently 0).
    pub nonce: u64,

    /// String IDs of peers that have verified this block.
    /// When `verifications.len() >= MIN_VERIFICATIONS` the block is finalised.
    pub verifications: Vec<String>,
}

impl Block {
    /// Compute the SHA-256 hash of this block's header fields (excluding
    /// `verifications`, which grow after block creation).
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.index.to_le_bytes());
        hasher.update(self.prev_hash);
        hasher.update(self.merkle_root);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.nonce.to_le_bytes());
        hasher.finalize().into()
    }

    /// Hex-encoded block hash.
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash())
    }

    /// Returns `true` when this block has been verified by at least
    /// [`MIN_VERIFICATIONS`] distinct peers.
    pub fn is_finalised(&self) -> bool {
        self.verifications.len() >= MIN_VERIFICATIONS
    }

    /// Record a peer verification.  Idempotent — duplicate peer IDs are
    /// ignored.  Returns `true` if the block just became finalised.
    pub fn add_verification(&mut self, peer_id: String) -> bool {
        if !self.verifications.contains(&peer_id) {
            self.verifications.push(peer_id);
        }
        self.is_finalised()
    }

    /// Build a new (non-genesis) block on top of a known previous hash.
    pub fn new(
        index: u64,
        prev_hash: [u8; 32],
        transactions: Vec<Transaction>,
    ) -> Result<Self, BlockchainError> {
        if transactions.is_empty() {
            return Err(BlockchainError::NoTransactions);
        }

        let tree = MerkleTree::new(&transactions)?;
        let merkle_root = tree.root_hash()?;

        Ok(Self {
            index,
            prev_hash,
            merkle_root,
            transactions,
            timestamp: Utc::now().timestamp(),
            nonce: 0,
            verifications: Vec::new(),
        })
    }

    /// Create the genesis block with a fixed all-zero previous hash.
    pub fn genesis() -> Self {
        let placeholder = Transaction::new(
            sp_transaction::TransactionType::NodeAdded,
            b"genesis".to_vec(),
        );

        let tree = MerkleTree::new(&[placeholder.clone()])
            .expect("genesis merkle tree should never fail");
        let merkle_root = tree.root_hash().expect("genesis root should exist");

        Self {
            index: 0,
            prev_hash: [0u8; 32],
            merkle_root,
            transactions: vec![placeholder],
            timestamp: 0,
            nonce: 0,
            verifications: Vec::new(),
        }
    }
}
