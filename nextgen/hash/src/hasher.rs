use sha2::{Digest, Sha256};
use starbase_utils::json::JsonError;
use tracing::{debug, trace};

pub trait ContentHashable: erased_serde::Serialize {}

erased_serde::serialize_trait_object!(ContentHashable);

pub struct ContentHasher<'owner> {
    content_cache: Option<String>,
    contents: Vec<&'owner dyn ContentHashable>,
    hash_cache: Option<String>,
    label: &'owner str,
}

unsafe impl<'owner> Send for ContentHasher<'owner> {}
unsafe impl<'owner> Sync for ContentHasher<'owner> {}

impl<'owner> ContentHasher<'owner> {
    pub fn new(label: &str) -> ContentHasher {
        debug!(label, "Created new content hasher");

        ContentHasher {
            content_cache: None,
            contents: Vec::new(),
            hash_cache: None,
            label,
        }
    }

    pub fn generate_hash(&mut self) -> miette::Result<String> {
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

    pub fn hash_content<T: ContentHashable>(&mut self, content: &'owner T) {
        trace!(label = self.label, "Adding content to hasher");

        self.contents.push(content);
        self.content_cache = None;
        self.hash_cache = None;
    }

    pub fn serialize(&mut self) -> miette::Result<&String> {
        if self.content_cache.is_none() {
            self.content_cache = Some(
                serde_json::to_string_pretty(&self.contents)
                    .map_err(|error| JsonError::Stringify { error })?,
            );
        }

        Ok(self.content_cache.as_ref().unwrap())
    }
}
