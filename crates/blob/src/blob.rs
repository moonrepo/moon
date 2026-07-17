use bytes::Bytes;
use miette::IntoDiagnostic;
use moon_hash::Digest;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub enum BlobContent {
    File(PathBuf),
    Inline(Bytes),
}

impl BlobContent {
    pub fn get_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Inline(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn get_size(&self) -> Option<usize> {
        match self {
            Self::Inline(bytes) => Some(bytes.len()),
            _ => None,
        }
    }

    pub fn read_bytes(&self) -> miette::Result<Vec<u8>> {
        match self {
            Self::Inline(bytes) => Ok(bytes.to_vec()),
            Self::File(path) => Ok(fs::read_file_bytes(path)?),
        }
    }
}

#[derive(Clone)]
pub struct BlobInput {
    pub content: BlobContent,
    pub digest: Digest,
}

impl BlobInput {
    pub fn into_blob(self) -> miette::Result<Blob> {
        Ok(Blob::new(self.digest, self.content.read_bytes()?))
    }
}

pub struct BlobOutput {
    pub content: BlobContent,
    pub digest: Digest,
}

impl From<Blob> for BlobOutput {
    fn from(blob: Blob) -> Self {
        BlobOutput {
            content: BlobContent::Inline(blob.bytes),
            digest: blob.digest,
        }
    }
}

impl Debug for BlobOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobOutput")
            .field("digest", &self.digest)
            .finish()
    }
}

#[derive(Clone)]
pub struct Blob {
    pub bytes: Bytes,
    pub digest: Digest,
}

impl Blob {
    pub fn new(digest: Digest, bytes: Vec<u8>) -> Self {
        Blob {
            digest,
            bytes: Bytes::from(bytes),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> miette::Result<Self> {
        Ok(Blob::new(Digest::from_bytes(&bytes)?, bytes))
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

impl TryFrom<Bytes> for Blob {
    type Error = miette::Report;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        Ok(Blob {
            digest: Digest::from_bytes(&bytes)?,
            bytes,
        })
    }
}

#[derive(Debug, Default)]
pub struct BlobCleanStats {
    pub blobs_removed: usize,
    pub bytes_saved: u64,
}
