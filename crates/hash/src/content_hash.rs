use crate::hash_error::HashError;
use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starbase_utils::hash;
use std::fmt;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

/// A SHA-256 content hash: 64-character hex string.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContentHash(Arc<CompactString>);

impl ContentHash {
    /// Hash a byte slice to produce a `ContentHash`.
    pub fn hash_bytes<T: AsRef<[u8]>>(bytes: T) -> miette::Result<Self> {
        ContentHash::from_hex(hash::sha256::from_bytes(bytes))
    }

    /// Hash a file's contents to produce a `ContentHash`.
    pub fn hash_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        // let mut hasher = blake3::Hasher::new();

        // let metadata = std::fs::metadata(path).map_err(|error| CasError::HashFailed {
        //     path: path.to_owned(),
        //     error: Box::new(error),
        // })?;

        // // Memory-map large files for fast hashing
        // if metadata.len() >= mmap_threshold {
        //     hasher
        //         .update_mmap(path)
        //         .map_err(|error| CasError::HashFailed {
        //             path: path.to_owned(),
        //             error: Box::new(error),
        //         })?;
        // } else {
        //     let bytes = fs::read_file_bytes(path)?;
        //     hasher.update(&bytes);
        // }

        ContentHash::from_hex(hash::sha256::from_file(path.as_ref())?)
    }

    /// Create a `ContentHash` from a hex string, validating format.
    pub fn from_hex<T: AsRef<str>>(hex: T) -> miette::Result<Self> {
        let hex = hex.as_ref();

        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(HashError::InvalidContentHash {
                hash: hex.to_owned(),
            }
            .into());
        }

        Ok(Self(Arc::new(hex.into())))
    }

    // /// Create a `ContentHash` from a BLAKE3 hash output.
    // pub fn from_hash(hash: blake3::Hash) -> Self {
    //     Self(hash.to_hex().to_string())
    // }

    /// The full 64-char hex digest.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The hash as a string slice.
    pub fn as_str(&self) -> &str {
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

// `Arc<CompactString>` doesn't implement serde, and we don't want to leak the
// `Arc` into the wire format anyway. Serialize as a plain hex string, exactly
// as the previous `ContentHash(CompactString)` newtype did, so existing cache
// manifests and action results stay readable.
impl Serialize for ContentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(Arc::new(CompactString::deserialize(deserializer)?)))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ContentHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for ContentHash {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
