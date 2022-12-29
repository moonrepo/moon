use crate::hasher::Hasher;
use serde::ser::{Serialize, SerializeSeq, Serializer};
use sha2::{Digest, Sha256};

pub struct HashSet {
    items: Vec<Box<dyn Hasher>>,
    sha: Option<Sha256>,
}

impl Default for HashSet {
    fn default() -> Self {
        HashSet {
            items: vec![],
            sha: Some(Sha256::default()),
        }
    }
}

impl HashSet {
    pub fn hash(&mut self, item: impl Hasher + 'static) {
        item.hash(self.sha.as_mut().unwrap());

        self.items.push(Box::new(item));
    }

    pub fn generate(&mut self) -> String {
        if self.items.is_empty() {
            return String::new();
        }

        let hash = format!("{:x}", self.sha.take().unwrap().finalize());

        // self.sha = Some(Sha256::default());

        hash
    }
}

impl Serialize for HashSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.items.len()))?;

        for item in &self.items {
            seq.serialize_element(&item.serialize())?;
        }

        seq.end()
    }
}
