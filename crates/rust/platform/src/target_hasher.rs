use moon_hasher::{Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTargetHasher {
    // Rust version
    deno_version: String,

    // All the dependencies (and their integrity hashes) of the project
    dependencies: BTreeMap<String, Vec<String>>,
}

impl RustTargetHasher {
    pub fn new(deno_version: Option<String>) -> Self {
        RustTargetHasher {
            deno_version: deno_version.unwrap_or_else(|| "unknown".into()),
            ..RustTargetHasher::default()
        }
    }

    pub fn hash_deps(&mut self, dependencies: BTreeMap<String, Vec<String>>) {
        self.dependencies = dependencies;
    }
}

impl Hasher for RustTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.deno_version.as_bytes());

        for versions in self.dependencies.values() {
            for version in versions {
                sha.update(version.as_bytes());
            }
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
