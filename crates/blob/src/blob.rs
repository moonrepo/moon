use bytes::Bytes;
use miette::IntoDiagnostic;
use moon_hash::Digest;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::fmt::Debug;
use std::path::Path;

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
