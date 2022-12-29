use crate::{hash_btree, Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsHasher {
    // Dependencies indexed by manifest name
    deps: BTreeMap<String, BTreeMap<String, String>>,

    // Version of our hasher
    #[allow(dead_code)]
    version: String,
}

impl DepsHasher {
    pub fn new() -> Self {
        DepsHasher {
            version: "1".into(),
            ..DepsHasher::default()
        }
    }

    pub fn hash_deps(&mut self, manifest: &str, deps: &BTreeMap<String, String>) {
        if let Some(cache) = self.deps.get_mut(manifest) {
            cache.extend(deps.to_owned());
        } else {
            self.deps.insert(manifest.to_owned(), deps.to_owned());
        }
    }
}

impl Hasher for DepsHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.version.as_bytes());

        for (manifest, deps) in &self.deps {
            sha.update(manifest.as_bytes());
            hash_btree(deps, sha);
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
