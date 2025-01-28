use serde::Serialize;
use sha2::{Digest, Sha256};
use starbase_utils::json;
use tracing::{debug, instrument, trace};

pub struct ContentHasher {
    content_cache: Option<String>,
    contents: Vec<String>,
    hash_cache: Option<String>,

    pub label: String,
}

unsafe impl Send for ContentHasher {}
unsafe impl Sync for ContentHasher {}

impl ContentHasher {
    pub fn new(label: &str) -> ContentHasher {
        trace!(label, "Created new content hasher");

        ContentHasher {
            content_cache: None,
            contents: Vec::new(),
            hash_cache: None,
            label: label.to_owned(),
        }
    }

    #[instrument(skip_all)]
    pub fn generate_hash(&mut self) -> miette::Result<String> {
        if let Some(hash) = &self.hash_cache {
            debug!(
                hash,
                label = &self.label,
                "Using cached content hash (previously generated)"
            );

            return Ok(hash.to_owned());
        }

        let mut hasher = Sha256::default();

        hasher.update(self.serialize()?.as_bytes());

        let hash = format!("{:x}", hasher.finalize());

        debug!(label = &self.label, hash, "Generated content hash");

        self.hash_cache = Some(hash.clone());

        Ok(hash)
    }

    pub fn hash_content<T: Serialize>(&mut self, content: T) -> miette::Result<()> {
        trace!(label = &self.label, "Adding content to hasher");

        self.contents.push(json::format(&content, false)?);
        self.content_cache = None;
        self.hash_cache = None;

        Ok(())
    }

    pub fn serialize(&mut self) -> miette::Result<&String> {
        if self.content_cache.is_none() {
            self.content_cache = Some(format!("[{}]", self.contents.join(",")));
        }

        Ok(self.content_cache.as_ref().unwrap())
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        match self.content_cache.take() {
            Some(data) => data.into_bytes(),
            None => vec![],
        }
    }
}
