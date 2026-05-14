use crate::content_hash::ContentHash;
use starbase_utils::fs;
use std::path::Path;

#[derive(Clone)]
pub struct Digest {
    pub hash: ContentHash,
    pub size_bytes: i64,
}

impl Digest {
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> miette::Result<Self> {
        let bytes = bytes.as_ref();
        let size_bytes = bytes.len() as i64;

        Ok(Digest {
            hash: ContentHash::hash_bytes(bytes)?,
            size_bytes,
        })
    }

    pub fn from_file<T: AsRef<Path>>(path: T) -> miette::Result<Self> {
        let bytes = fs::read_file_bytes(path.as_ref())?;

        Self::from_bytes(&bytes)
    }
}
