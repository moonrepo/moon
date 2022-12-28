use crate::hasher::Hasher;
use sha2::{Digest, Sha256};

pub struct HashSet {
    pub items: Vec<Box<dyn Hasher>>,
    sha: Sha256,
}

impl HashSet {
    pub fn hash(&mut self, item: impl Hasher + 'static) -> &mut Self {
        item.hash(&mut self.sha);
        self.items.push(Box::new(item));
        self
    }

    pub fn generate(self) -> String {
        format!("{:x}", self.sha.finalize())
    }
}
