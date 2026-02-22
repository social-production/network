use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("transport error: {0}")]
    Transport(String),

    #[error("gossipsub error: {0}")]
    Gossipsub(String),

    #[error("serialisation error: {0}")]
    Serialisation(String),

    #[error("blockchain error: {0}")]
    Blockchain(#[from] sp_blockchain::BlockchainError),

    #[error("sync error: {0}")]
    Sync(#[from] sp_sync::SyncError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("no pending transactions to form a block")]
    NoPendingTransactions,
}
