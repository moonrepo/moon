use crate::hash_error::HashError;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::{debug, trace};

pub struct ContentHasher<'owner, T: Serialize> {
    content_cache: Option<String>,
    contents: Vec<T>,
    hash_cache: Option<String>,
    label: &'owner str,
}

impl<'owner, T: Serialize> ContentHasher<'owner, T> {
    pub fn new<'new>(label: &'new str) -> ContentHasher<'new, T> {
        debug!(label, "Created new content hasher");

        ContentHasher {
            content_cache: None,
            contents: Vec::new(),
            hash_cache: None,
            label,
        }
    }

    pub fn generate(&mut self) -> Result<String, HashError> {
        if let Some(hash) = &self.hash_cache {
            debug!(
                hash,
                label = self.label,
                "Using cached content hash (previously generated)"
            );

            return Ok(hash.to_owned());
        }

        let mut hasher = Sha256::default();

        hasher.update(self.serialize()?.as_bytes());

        let hash = format!("{:x}", hasher.finalize());

        debug!(hash, label = self.label, "Generated content hash");

        self.hash_cache = Some(hash.clone());

        Ok(hash)
    }

    pub fn hash(&mut self, content: T) {
        trace!(label = self.label, "Hashing content");

        self.contents.push(content);
        self.content_cache = None;
        self.hash_cache = None;
    }

    pub fn serialize(&mut self) -> Result<&String, HashError> {
        if self.content_cache.is_none() {
            self.content_cache = Some(serde_json::to_string_pretty(&self.contents).map_err(
                |error| HashError::ContentHashFailed {
                    error,
                    label: self.label.to_owned(),
                },
            )?);
        }

        Ok(self.content_cache.as_ref().unwrap())
    }
}
