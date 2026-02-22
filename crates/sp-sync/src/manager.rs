use sp_blockchain::{Block, Blockchain};

use crate::{SyncError, SyncStrategy};

/// Applies a [`SyncStrategy`] to decide which blocks should be requested from
/// remote peers and how to merge an incoming chain segment.
pub struct SyncManager {
    strategy: SyncStrategy,
    /// Estimated bytes of blockchain data already downloaded in this session.
    downloaded_bytes: u64,
}

impl SyncManager {
    pub fn new(strategy: SyncStrategy) -> Self {
        Self {
            strategy,
            downloaded_bytes: 0,
        }
    }

    pub fn strategy(&self) -> &SyncStrategy {
        &self.strategy
    }

    pub fn set_strategy(&mut self, strategy: SyncStrategy) {
        self.strategy = strategy;
        self.downloaded_bytes = 0;
    }

    /// Given the remote peer's chain, return the slice of blocks that should
    /// be applied locally according to the active strategy.
    ///
    /// The `local` chain is used to determine the starting point (we only look
    /// at blocks the local node does not yet have).
    pub fn blocks_to_sync<'a>(
        &mut self,
        local: &Blockchain,
        remote_blocks: &'a [Block],
    ) -> Result<Vec<&'a Block>, SyncError> {
        let local_len = local.len() as u64;

        // Only consider blocks beyond our current tip.
        let new_blocks: Vec<&Block> = remote_blocks
            .iter()
            .filter(|b| b.index >= local_len)
            .collect();

        match &self.strategy {
            SyncStrategy::OnDemand => Ok(Vec::new()),

            SyncStrategy::TimeRange { from, to } => {
                if from > to {
                    return Err(SyncError::InvalidTimeRange);
                }
                Ok(new_blocks
                    .into_iter()
                    .filter(|b| b.timestamp >= *from && b.timestamp <= *to)
                    .collect())
            }

            SyncStrategy::SizeLimit { max_bytes } => {
                let mut selected = Vec::new();
                for block in new_blocks {
                    let estimated = estimated_block_size(block);
                    if self.downloaded_bytes + estimated > *max_bytes {
                        break;
                    }
                    self.downloaded_bytes += estimated;
                    selected.push(block);
                }
                Ok(selected)
            }
        }
    }

    /// Record that a specific block has been downloaded (used by callers that
    /// handle on-demand requests to keep the byte counter accurate).
    pub fn record_download(&mut self, block: &Block) {
        self.downloaded_bytes += estimated_block_size(block);
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes
    }
}

/// Rough byte estimate for a block: sum of serialised transaction payload sizes
/// plus a fixed header overhead.
fn estimated_block_size(block: &Block) -> u64 {
    let payload_bytes: usize = block.transactions.iter().map(|tx| tx.payload.len()).sum();
    // 256 bytes overhead per block for header fields (hashes, index, timestamp).
    (payload_bytes + 256) as u64
}

#[cfg(test)]
mod tests {
    use sp_blockchain::Blockchain;
    use sp_transaction::{Transaction, TransactionType};

    use super::*;

    fn make_chain_with_blocks(count: usize) -> Blockchain {
        let mut chain = Blockchain::new();
        for _ in 0..count {
            chain
                .add_block(vec![Transaction::new(
                    TransactionType::PostCreated,
                    b"hello".to_vec(),
                )])
                .unwrap();
        }
        chain
    }

    #[test]
    fn on_demand_returns_no_blocks() {
        let local = Blockchain::new();
        let remote = make_chain_with_blocks(3);
        let mut mgr = SyncManager::new(SyncStrategy::OnDemand);
        let result = mgr.blocks_to_sync(&local, remote.blocks()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn size_limit_caps_downloaded_blocks() {
        let local = Blockchain::new();
        let remote = make_chain_with_blocks(10);

        // Each block has payload ~5 bytes + 256 overhead = 261 bytes.
        // Limit of 600 bytes should let through at most 2 blocks.
        let mut mgr = SyncManager::new(SyncStrategy::SizeLimit { max_bytes: 600 });
        let blocks = mgr.blocks_to_sync(&local, remote.blocks()).unwrap();
        assert!(blocks.len() <= 2);
    }

    #[test]
    fn time_range_filters_by_timestamp() {
        let local = Blockchain::new();
        let remote = make_chain_with_blocks(3);

        // All blocks in the remote chain have a real timestamp; set a range
        // that excludes all of them (far future).
        let mut mgr = SyncManager::new(SyncStrategy::TimeRange {
            from: i64::MAX - 1,
            to: i64::MAX,
        });
        let blocks = mgr.blocks_to_sync(&local, remote.blocks()).unwrap();
        assert!(blocks.is_empty());
    }

    #[test]
    fn invalid_time_range_returns_error() {
        let local = Blockchain::new();
        let remote = make_chain_with_blocks(1);
        let mut mgr = SyncManager::new(SyncStrategy::TimeRange { from: 100, to: 50 });
        assert!(mgr.blocks_to_sync(&local, remote.blocks()).is_err());
    }
}
