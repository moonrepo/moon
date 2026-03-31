use crate::cas_error::CasError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A BLAKE3 content hash: 64-character lowercase hex string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Parse and validate a hex string as a content hash.
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
    pub(crate) fn from_blake3(hash: blake3::Hash) -> Self {
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
