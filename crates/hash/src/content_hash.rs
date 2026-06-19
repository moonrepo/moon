use crate::hash_error::HashError;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starbase_utils::fs::{self, FsError};
use std::fmt;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;

pub fn hash_sha256<T: AsRef<[u8]>>(bytes: T) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A SHA-256 content hash: 64-character hex string.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct ContentHash(CompactString);

impl ContentHash {
    /// Hash a byte slice to produce a `ContentHash`.
    pub fn hash_bytes<T: AsRef<[u8]>>(bytes: T) -> miette::Result<Self> {
        ContentHash::from_hex(hash_sha256(bytes))
    }

    /// Hash a file's contents to produce a `ContentHash`.
    pub fn hash_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        let path = path.as_ref();
        let mut sha = Sha256::new();
        let mut file = fs::open_file(path)?;
        let mut buffer = [0u8; 64 * 1024];

        // Read in chunks instead of pulling the entire file into memory
        loop {
            let n = file.read(&mut buffer).map_err(|error| FsError::Read {
                path: path.to_path_buf(),
                error: Box::new(error),
            })?;

            if n == 0 {
                break;
            }

            sha.update(&buffer[..n]);
        }

        // let mut hasher = blake3::Hasher::new();

        // // Note: don't use starbase as it logs too much!
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

        ContentHash::from_hex(hex::encode(sha.finalize()))
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

        Ok(Self(hex.into()))
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
