use crate::{hash_btree, Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsHasher {
    dependencies: BTreeMap<String, String>,

    name: String,
}

impl DepsHasher {
    pub fn new(name: String) -> Self {
        DepsHasher {
            name,
            ..DepsHasher::default()
        }
    }

    pub fn hash_dep<K: AsRef<str>, V: AsRef<str>>(&mut self, name: K, value: V) {
        self.dependencies
            .insert(name.as_ref().to_owned(), value.as_ref().to_owned());
    }

    pub fn hash_deps(&mut self, dependencies: &BTreeMap<String, String>) {
        self.dependencies.extend(dependencies.to_owned());
    }
}

impl Hasher for DepsHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.name.as_bytes());
        hash_btree(&self.dependencies, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
