use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("invalid time range: from > to")]
    InvalidTimeRange,

    #[error("blockchain error: {0}")]
    Blockchain(#[from] sp_blockchain::BlockchainError),
}
