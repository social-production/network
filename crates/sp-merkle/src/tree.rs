use sha2::{Digest, Sha256};
use sp_transaction::Transaction;
use uuid::Uuid;

use crate::{
    error::MerkleError,
    proof::{MerkleProof, ProofNode, ProofSide},
};

/// A binary Merkle tree built from a slice of [`Transaction`]s.
///
/// Leaf hashes are the SHA-256 digests of each serialised transaction.
/// Parent hashes are SHA-256 of `left_child || right_child`.
/// When the number of leaves is odd the last leaf is duplicated so that every
/// level is always even-width.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All levels of the tree, `levels[0]` = leaf hashes,
    /// `levels[last]` = single root hash.
    levels: Vec<Vec<[u8; 32]>>,
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut v = left.to_vec();
    v.extend_from_slice(right);
    Sha256::digest(&v).into()
}

impl MerkleTree {
    /// Build a Merkle tree from `transactions`.  Returns an error if the slice
    /// is empty or any transaction cannot be hashed.
    pub fn new(transactions: &[Transaction]) -> Result<Self, MerkleError> {
        if transactions.is_empty() {
            return Err(MerkleError::Empty);
        }

        let mut leaves: Vec<[u8; 32]> = transactions
            .iter()
            .map(|tx| tx.hash().map_err(|e| MerkleError::HashFailed(e.to_string())))
            .collect::<Result<_, _>>()?;

        let mut levels: Vec<Vec<[u8; 32]>> = vec![leaves.clone()];

        while leaves.len() > 1 {
            // Duplicate last leaf when odd number of leaves.
            if leaves.len() % 2 != 0 {
                let last = *leaves.last().unwrap();
                leaves.push(last);
            }

            let parent_level: Vec<[u8; 32]> = leaves
                .chunks(2)
                .map(|chunk| hash_pair(&chunk[0], &chunk[1]))
                .collect();

            levels.push(parent_level.clone());
            leaves = parent_level;
        }

        Ok(Self { levels })
    }

    /// The Merkle root hash.  Returns an error if the tree is empty (shouldn't
    /// happen after a successful `new` call, but guarded for safety).
    pub fn root_hash(&self) -> Result<[u8; 32], MerkleError> {
        self.levels
            .last()
            .and_then(|l| l.first())
            .copied()
            .ok_or(MerkleError::Empty)
    }

    /// Hex-encoded root hash.
    pub fn root_hash_hex(&self) -> Result<String, MerkleError> {
        Ok(hex::encode(self.root_hash()?))
    }

    /// Build an inclusion proof for the transaction with the given `tx_id`.
    /// Searches leaf level by matching the transaction hash stored in `leaves`.
    ///
    /// The caller must supply `transactions` (same slice used to build the tree)
    /// so we can resolve `tx_id` → leaf index.
    pub fn proof(
        &self,
        transactions: &[Transaction],
        tx_id: Uuid,
    ) -> Result<MerkleProof, MerkleError> {
        // Find the leaf index that corresponds to tx_id.
        let leaf_index = transactions
            .iter()
            .position(|tx| tx.id == tx_id)
            .ok_or(MerkleError::NotFound)?;

        let leaf_hash = self.levels[0][leaf_index];
        let mut path = Vec::new();
        let mut index = leaf_index;

        for level in &self.levels[..self.levels.len() - 1] {
            // Ensure the level is padded to even length (mirrors build logic).
            let mut padded = level.clone();
            if padded.len() % 2 != 0 {
                let last = *padded.last().unwrap();
                padded.push(last);
            }

            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            let sibling_hash = padded[sibling_index];
            let side = if index % 2 == 0 {
                ProofSide::Right
            } else {
                ProofSide::Left
            };

            path.push(ProofNode {
                hash: sibling_hash,
                side,
            });

            index /= 2;
        }

        Ok(MerkleProof { leaf_hash, path })
    }
}

#[cfg(test)]
mod tests {
    use sp_transaction::{Transaction, TransactionType};

    use super::*;

    fn make_tx(kind: TransactionType, payload: &[u8]) -> Transaction {
        Transaction::new(kind, payload.to_vec())
    }

    #[test]
    fn single_transaction_root_equals_leaf_hash() {
        let tx = make_tx(TransactionType::UserRegistered, b"user1");
        let tree = MerkleTree::new(&[tx.clone()]).unwrap();
        assert_eq!(tree.root_hash().unwrap(), tx.hash().unwrap());
    }

    #[test]
    fn empty_transactions_returns_error() {
        assert!(MerkleTree::new(&[]).is_err());
    }

    #[test]
    fn proof_verifies_correctly() {
        let txs: Vec<Transaction> = (0..4)
            .map(|i| make_tx(TransactionType::PostCreated, &[i]))
            .collect();

        let tree = MerkleTree::new(&txs).unwrap();
        let root = tree.root_hash().unwrap();

        for tx in &txs {
            let proof = tree.proof(&txs, tx.id).unwrap();
            assert!(proof.verify(&root), "proof failed for tx {}", tx.id);
        }
    }

    #[test]
    fn proof_verifies_odd_number_of_transactions() {
        let txs: Vec<Transaction> = (0..5)
            .map(|i| make_tx(TransactionType::VoteCast, &[i]))
            .collect();

        let tree = MerkleTree::new(&txs).unwrap();
        let root = tree.root_hash().unwrap();

        for tx in &txs {
            let proof = tree.proof(&txs, tx.id).unwrap();
            assert!(proof.verify(&root), "proof failed for tx {}", tx.id);
        }
    }

    #[test]
    fn tampered_proof_fails_verification() {
        let txs: Vec<Transaction> = (0..4)
            .map(|i| make_tx(TransactionType::NodeAdded, &[i]))
            .collect();

        let tree = MerkleTree::new(&txs).unwrap();
        let mut wrong_root = tree.root_hash().unwrap();
        wrong_root[0] ^= 0xff;

        let proof = tree.proof(&txs, txs[0].id).unwrap();
        assert!(!proof.verify(&wrong_root));
    }
}
