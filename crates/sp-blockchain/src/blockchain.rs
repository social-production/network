use serde::{Deserialize, Serialize};
use sp_transaction::Transaction;

use crate::{block::Block, error::BlockchainError};

/// The append-only chain of [`Block`]s that forms the Social Production ledger.
///
/// Invariants maintained by this type:
/// - Always contains at least the genesis block.
/// - Every block's `prev_hash` matches the hash of the preceding block.
/// - Block indices are contiguous starting from 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl Blockchain {
    /// Initialise a new chain with only the genesis block.
    pub fn new() -> Self {
        Self {
            blocks: vec![Block::genesis()],
        }
    }

    /// Number of blocks in the chain (including genesis).
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// The most recent block.
    pub fn tip(&self) -> &Block {
        // Safety: always at least one block (genesis).
        self.blocks.last().unwrap()
    }

    /// Append a new block containing `transactions`.
    ///
    /// The new block's `prev_hash` is set to the current tip's hash.
    pub fn add_block(&mut self, transactions: Vec<Transaction>) -> Result<&Block, BlockchainError> {
        let prev_hash = self.tip().hash();
        let index = self.tip().index + 1;
        let block = Block::new(index, prev_hash, transactions)?;
        self.blocks.push(block);
        Ok(self.blocks.last().unwrap())
    }

    /// Record a peer verification for the block at `block_index`.
    ///
    /// Returns `true` if the block just reached [`MIN_VERIFICATIONS`].
    pub fn verify_block(
        &mut self,
        block_index: u64,
        peer_id: String,
    ) -> Result<bool, BlockchainError> {
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.index == block_index)
            .ok_or(BlockchainError::BlockNotFound(block_index))?;

        Ok(block.add_verification(peer_id))
    }

    /// Return a reference to a block by its index.
    pub fn get_block(&self, index: u64) -> Option<&Block> {
        self.blocks.get(index as usize)
    }

    /// Return all blocks from `start_index` onward (inclusive).
    pub fn blocks_from(&self, start_index: u64) -> &[Block] {
        let pos = start_index as usize;
        if pos >= self.blocks.len() {
            &[]
        } else {
            &self.blocks[pos..]
        }
    }

    /// All blocks in the chain.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Validate the full chain structure:
    /// - Each block's `prev_hash` matches the hash of the previous block.
    /// - Block indices are contiguous.
    pub fn is_valid(&self) -> bool {
        if self.blocks.is_empty() {
            return false;
        }

        for window in self.blocks.windows(2) {
            let prev = &window[0];
            let next = &window[1];

            if next.prev_hash != prev.hash() {
                return false;
            }
            if next.index != prev.index + 1 {
                return false;
            }
        }

        true
    }

    /// Replace the local chain with `other` if `other` is longer and valid.
    ///
    /// This is the simple longest-chain conflict resolution rule used during
    /// peer sync.
    pub fn sync_from(&mut self, other: &Blockchain) -> bool {
        if other.len() > self.len() && other.is_valid() {
            *self = other.clone();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use sp_transaction::{Transaction, TransactionType};

    use super::*;

    fn tx(kind: TransactionType) -> Transaction {
        Transaction::new(kind, b"test".to_vec())
    }

    #[test]
    fn new_chain_is_valid() {
        assert!(Blockchain::new().is_valid());
    }

    #[test]
    fn add_block_extends_chain() {
        let mut chain = Blockchain::new();
        chain.add_block(vec![tx(TransactionType::UserRegistered)]).unwrap();
        assert_eq!(chain.len(), 2);
        assert!(chain.is_valid());
    }

    #[test]
    fn verify_block_tracks_peers() {
        let mut chain = Blockchain::new();
        chain.add_block(vec![tx(TransactionType::ProjectPosted)]).unwrap();

        assert!(!chain.verify_block(1, "peer-a".into()).unwrap());
        assert!(!chain.verify_block(1, "peer-b".into()).unwrap());
        // Third verification finalises the block.
        assert!(chain.verify_block(1, "peer-c".into()).unwrap());
        // Duplicate peer should not re-trigger finalisation flag.
        assert!(chain.verify_block(1, "peer-c".into()).unwrap());
        assert!(chain.get_block(1).unwrap().is_finalised());
    }

    #[test]
    fn sync_from_longer_valid_chain() {
        let mut local = Blockchain::new();
        let mut remote = Blockchain::new();

        remote.add_block(vec![tx(TransactionType::NodeAdded)]).unwrap();
        remote.add_block(vec![tx(TransactionType::NodeAdded)]).unwrap();

        assert!(local.sync_from(&remote));
        assert_eq!(local.len(), remote.len());
    }

    #[test]
    fn sync_from_shorter_chain_ignored() {
        let mut local = Blockchain::new();
        local.add_block(vec![tx(TransactionType::NodeAdded)]).unwrap();

        let shorter = Blockchain::new();
        assert!(!local.sync_from(&shorter));
        assert_eq!(local.len(), 2);
    }
}
