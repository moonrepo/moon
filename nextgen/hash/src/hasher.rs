use crate::hash_error::HashError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::{debug, trace};

pub struct ContentHasher<'owner, T: Serialize> {
    cache: Option<String>,
    contents: Vec<T>,
    label: &'owner str,
}

impl<'owner, T: Serialize> ContentHasher<'owner, T> {
    pub fn new<'new>(label: &'new str) -> ContentHasher<'new, T> {
        debug!(label, "Created new content hasher");

        ContentHasher {
            cache: None,
            contents: Vec::new(),
            label,
        }
    }

    pub fn generate(&mut self) -> Result<String, HashError> {
        let mut hasher = Sha256::default();

        hasher.update(self.serialize()?.as_bytes());

        let hash = format!("{:x}", hasher.finalize());

        debug!(hash, label = self.label, "Generated content hash");

        Ok(hash)
    }

    pub fn hash(&mut self, content: T) {
        trace!(label = self.label, "Hashing content");

        self.cache = None;
        self.contents.push(content);
    }

    pub fn serialize(&mut self) -> Result<&String, HashError> {
        if self.cache.is_none() {
            self.cache = Some(
                serde_json::to_string_pretty(&self.contents).map_err(|error| {
                    HashError::ContentHashFailed {
                        error,
                        label: self.label.to_owned(),
                    }
                })?,
            );
        }

        Ok(self.cache.as_ref().unwrap())
    }
}
