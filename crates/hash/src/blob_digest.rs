use crate::content_hash::ContentHash;
use miette::IntoDiagnostic;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::fmt::Debug;
use std::path::Path;

#[derive(Clone)]
pub struct Blob {
    pub bytes: Vec<u8>,
    pub digest: Digest,
}

impl Blob {
    pub fn from_bytes(bytes: Vec<u8>) -> miette::Result<Self> {
        Ok(Blob {
            digest: Digest {
                hash: ContentHash::hash_bytes(&bytes)?,
                size: bytes.len() as i64,
            },
            bytes,
        })
    }

    pub fn from_data<T: Serialize>(data: T) -> miette::Result<Self> {
        Self::from_bytes(json::serde_json::to_vec(&data).into_diagnostic()?)
    }

    pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        Self::from_bytes(fs::read_file_bytes(path.as_ref())?)
    }
}

impl Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Blob")
            .field("digest", &self.digest)
            .finish()
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Digest {
    pub hash: ContentHash,
    pub size: i64,
}

impl Digest {
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> miette::Result<Self> {
        let bytes = bytes.as_ref();
        let size = bytes.len() as i64;

        Ok(Digest {
            hash: ContentHash::hash_bytes(bytes)?,
            size,
        })
    }

    pub fn from_data<T: Serialize>(data: T) -> miette::Result<Self> {
        let bytes = json::serde_json::to_vec(&data).into_diagnostic()?;

        Self::from_bytes(&bytes)
    }

    pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        let bytes = fs::read_file_bytes(path.as_ref())?;

        Self::from_bytes(&bytes)
    }
}
