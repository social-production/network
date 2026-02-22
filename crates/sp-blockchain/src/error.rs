use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("chain is empty")]
    Empty,

    #[error("block index {0} not found")]
    BlockNotFound(u64),

    #[error("invalid chain: {0}")]
    InvalidChain(String),

    #[error("merkle error: {0}")]
    Merkle(#[from] sp_merkle::MerkleError),

    #[error("transaction error: {0}")]
    Transaction(#[from] sp_transaction::TransactionError),

    #[error("no transactions supplied for new block")]
    NoTransactions,
}
