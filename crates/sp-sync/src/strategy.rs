use serde::{Deserialize, Serialize};

/// Controls which blocks (and their embedded assets) a node will request from
/// its peers.
///
/// This directly implements the "Resource usage is controllable" requirement
/// from the architecture spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStrategy {
    /// Sync all blocks whose transactions fall within the given Unix-timestamp
    /// window (inclusive on both ends).
    TimeRange { from: i64, to: i64 },

    /// Stop syncing once the estimated on-disk size of downloaded blocks
    /// exceeds `max_bytes`.
    SizeLimit { max_bytes: u64 },

    /// Do not proactively request blocks; only sync when the application
    /// explicitly requests a specific block or transaction.
    OnDemand,
}

impl Default for SyncStrategy {
    /// Default to syncing everything (no restriction).
    fn default() -> Self {
        Self::OnDemand
    }
}
