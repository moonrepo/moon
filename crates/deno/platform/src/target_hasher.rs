use moon_hasher::{hash_btree, Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DenoTargetHasher {
    // Deno version
    deno_version: String,

    // All the dependencies of the project (including dev and peer),
    // and the hashes corresponding with their versions
    dependencies: BTreeMap<String, Vec<String>>,

    // Version of our hasher
    #[allow(dead_code)]
    version: String,
}

impl DenoTargetHasher {
    pub fn new(deno_version: Option<String>) -> Self {
        DenoTargetHasher {
            deno_version: deno_version.unwrap_or_else(|| "unknown".into()),
            version: "1".into(),
            ..DenoTargetHasher::default()
        }
    }
}

impl Hasher for DenoTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.version.as_bytes());
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
