use crate::hash_error::HashError;
use sha2::{Digest, Sha256};
use tracing::{debug, trace};

pub trait ContentHashable: erased_serde::Serialize {}

erased_serde::serialize_trait_object!(ContentHashable);

pub struct ContentHasher<'owner> {
    content_cache: Option<String>,
    contents: Vec<Box<dyn ContentHashable>>,
    hash_cache: Option<String>,
    label: &'owner str,
}

impl<'owner> ContentHasher<'owner> {
    pub fn new<'new>(label: &'new str) -> ContentHasher<'new> {
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

    pub fn hash(&mut self, content: impl ContentHashable + 'static) {
        trace!(label = self.label, "Hashing content");

        self.contents.push(Box::new(content));
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
