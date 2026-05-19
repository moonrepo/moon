use crate::content_hash::ContentHash;
use moon_common::path::WorkspaceRelativePathBuf;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

    pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        Self::from_bytes(fs::read_file_bytes(path.as_ref())?)
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

    pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        let bytes = fs::read_file_bytes(path.as_ref())?;

        Self::from_bytes(&bytes)
    }
}

pub type OutputBlobs = BTreeMap<PathBuf, Blob>;
pub type OutputHashes = BTreeMap<WorkspaceRelativePathBuf, ContentHash>;
