use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid transaction id")]
    InvalidId,
}
