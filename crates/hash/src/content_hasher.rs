use crate::content_hash::ContentHash;
use serde::Serialize;
use starbase_utils::json;
use tracing::{debug, instrument};

pub struct ContentHasher {
    content_cache: Option<String>,
    contents: Vec<String>,
    hash_cache: Option<ContentHash>,

    pub label: String,
}

unsafe impl Send for ContentHasher {}
unsafe impl Sync for ContentHasher {}

impl ContentHasher {
    pub fn new(label: &str) -> ContentHasher {
        debug!(label, "Created new content hasher");

        ContentHasher {
            content_cache: None,
            contents: Vec::new(),
            hash_cache: None,
            label: label.to_owned(),
        }
    }

    #[instrument(skip_all)]
    pub fn generate_hash(&mut self) -> miette::Result<ContentHash> {
        if let Some(hash) = &self.hash_cache {
            return Ok(hash.to_owned());
        }

        let hash = ContentHash::hash_bytes(self.serialize()?)?;

        debug!(
            label = &self.label,
            hash = hash.as_str(),
            "Generated content hash"
        );

        self.hash_cache = Some(hash.clone());

        Ok(hash)
    }

    pub fn hash_content<T: Serialize>(&mut self, content: T) -> miette::Result<()> {
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
