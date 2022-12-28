use crate::hasher::Hasher;
use serde::ser::{Serialize, SerializeSeq, Serializer};
use sha2::{Digest, Sha256};

#[derive(Default)]
pub struct HashSet {
    items: Vec<Box<dyn Hasher>>,
    sha: Option<Sha256>,
}

impl HashSet {
    pub fn hash(&mut self, item: impl Hasher + 'static) -> &mut Self {
        item.hash(self.sha.as_mut().unwrap());
        self.items.push(Box::new(item));
        self
    }

    pub fn generate(&mut self) -> String {
        format!("{:x}", self.sha.take().unwrap().finalize())
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
