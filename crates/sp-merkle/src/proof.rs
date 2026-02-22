use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Indicates which side the sibling hash sits on when re-computing a parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSide {
    Left,
    Right,
}

/// A single step in a Merkle inclusion proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofNode {
    pub hash: [u8; 32],
    pub side: ProofSide,
}

/// An inclusion proof for a single transaction leaf.
///
/// Verify by hashing the leaf upward through each sibling until the computed
/// root matches the expected root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The hash of the leaf being proved.
    pub leaf_hash: [u8; 32],
    /// Ordered list of sibling hashes from leaf to root.
    pub path: Vec<ProofNode>,
}

impl MerkleProof {
    /// Returns `true` if following the proof path reproduces `expected_root`.
    pub fn verify(&self, expected_root: &[u8; 32]) -> bool {
        let mut current = self.leaf_hash;

        for node in &self.path {
            let combined = match node.side {
                ProofSide::Left => {
                    let mut v = node.hash.to_vec();
                    v.extend_from_slice(&current);
                    v
                }
                ProofSide::Right => {
                    let mut v = current.to_vec();
                    v.extend_from_slice(&node.hash);
                    v
                }
            };
            current = Sha256::digest(&combined).into();
        }

        &current == expected_root
    }
}
