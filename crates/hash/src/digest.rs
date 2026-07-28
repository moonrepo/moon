use crate::content_hash::ContentHash;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, json};
use std::fmt::Debug;
use std::ops::Deref;
use std::path::Path;

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
        let metadata = fs::metadata(path.as_ref())?;
        let size = metadata.len() as i64;

        Ok(Digest {
            hash: ContentHash::hash_file(path)?,
            size,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.size >= 0 && !self.hash.is_empty()
    }
}

impl AsRef<ContentHash> for Digest {
    fn as_ref(&self) -> &ContentHash {
        &self.hash
    }
}

impl AsRef<str> for Digest {
    fn as_ref(&self) -> &str {
        &self.hash
    }
}

impl Deref for Digest {
    type Target = ContentHash;

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}
