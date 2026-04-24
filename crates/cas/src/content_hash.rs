use crate::cas_error::CasError;
use serde::{Deserialize, Serialize};
use starbase_utils::fs;
use std::fmt;
use std::path::Path;

/// A BLAKE3 content hash: 64-character lowercase hex string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Hash a byte slice to produce a `ContentHash`.
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        ContentHash::from_hash(blake3::hash(bytes))
    }

    /// Hash a file's contents to produce a `ContentHash`.
    pub fn hash_file(path: &Path, mmap_threshold: u64) -> miette::Result<Self> {
        let mut hasher = blake3::Hasher::new();

        // Note: don't use starbase as it logs too much!
        let metadata = std::fs::metadata(path).map_err(|error| CasError::HashFailed {
            path: path.to_owned(),
            error: Box::new(error),
        })?;

        // Memory-map large files for fast hashing
        if metadata.len() >= mmap_threshold {
            hasher
                .update_mmap(path)
                .map_err(|error| CasError::HashFailed {
                    path: path.to_owned(),
                    error: Box::new(error),
                })?;
        } else {
            let bytes = fs::read_file_bytes(path)?;
            hasher.update(&bytes);
        }

        Ok(ContentHash::from_hash(hasher.finalize()))
    }

    /// Create a `ContentHash` from a hex string, validating format.
    pub fn from_hex(hex: &str) -> miette::Result<Self> {
        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CasError::InvalidHash {
                hash: hex.to_owned(),
            }
            .into());
        }

        Ok(Self(hex.to_ascii_lowercase()))
    }

    /// Create a `ContentHash` from a BLAKE3 hash output.
    pub fn from_hash(hash: blake3::Hash) -> Self {
        Self(hash.to_hex().to_string())
    }

    /// The full 64-char hex digest.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// First 2 hex chars (shard directory name).
    pub fn prefix(&self) -> &str {
        &self.0[..2]
    }

    /// Remaining 62 hex chars (blob filename).
    pub fn suffix(&self) -> &str {
        &self.0[2..]
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
