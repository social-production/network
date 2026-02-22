use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{TransactionError, TransactionType};

/// A single immutable record of an event on the Social Production network.
///
/// The `payload` field carries JSON-encoded domain data (user profile, project
/// details, etc.) so that this crate stays domain-agnostic while still being
/// fully serialisable.
///
/// The `signature` field is reserved for a cryptographic signature that higher-
/// level code (e.g. `sp-node`) can populate once key management is in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique identifier for this transaction.
    pub id: Uuid,

    /// The kind of event this transaction records.
    pub kind: TransactionType,

    /// JSON-encoded domain payload.
    pub payload: Vec<u8>,

    /// Unix timestamp (seconds) when this transaction was created.
    pub timestamp: i64,

    /// Cryptographic signature of `id || kind || payload || timestamp`.
    /// Empty until signed by the originating node.
    pub signature: Vec<u8>,
}

impl Transaction {
    /// Create a new unsigned transaction.
    pub fn new(kind: TransactionType, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            payload,
            timestamp: Utc::now().timestamp(),
            signature: Vec::new(),
        }
    }

    /// Compute the SHA-256 hash of the canonical byte representation of this
    /// transaction.  Used as the leaf value in the Merkle tree.
    pub fn hash(&self) -> Result<[u8; 32], TransactionError> {
        let bytes = bincode::serialize(self)?;
        let digest = Sha256::digest(&bytes);
        Ok(digest.into())
    }

    /// Hex-encoded hash, useful for display and logging.
    pub fn hash_hex(&self) -> Result<String, TransactionError> {
        Ok(hex::encode(self.hash()?))
    }

    /// Attach a pre-computed signature (e.g. from an ed25519 keypair).
    pub fn sign(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }

    /// True when a signature has been attached.
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_transaction_has_unique_ids() {
        let a = Transaction::new(TransactionType::UserRegistered, b"payload-a".to_vec());
        let b = Transaction::new(TransactionType::UserRegistered, b"payload-b".to_vec());
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn hash_is_deterministic_for_same_data() {
        let tx = Transaction {
            id: Uuid::nil(),
            kind: TransactionType::ProjectPosted,
            payload: b"hello".to_vec(),
            timestamp: 0,
            signature: vec![],
        };
        assert_eq!(tx.hash().unwrap(), tx.hash().unwrap());
    }

    #[test]
    fn different_payloads_produce_different_hashes() {
        let make = |p: &[u8]| Transaction {
            id: Uuid::nil(),
            kind: TransactionType::PostCreated,
            payload: p.to_vec(),
            timestamp: 0,
            signature: vec![],
        };
        assert_ne!(make(b"a").hash().unwrap(), make(b"b").hash().unwrap());
    }
}
