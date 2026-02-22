pub mod block;
pub mod blockchain;
pub mod error;

pub use block::Block;
pub use blockchain::Blockchain;
pub use error::BlockchainError;

/// Minimum number of distinct peer verifications required before a block is
/// considered finalised.  Derived directly from the architecture spec.
pub const MIN_VERIFICATIONS: usize = 3;
