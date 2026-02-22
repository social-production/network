use thiserror::Error;

#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("tree is empty")]
    Empty,

    #[error("transaction not found in tree")]
    NotFound,

    #[error("transaction hashing failed: {0}")]
    HashFailed(String),
}
