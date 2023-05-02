use moon_hasher::{Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustTargetHasher {
    pub rust_version: String,
    pub locked_dependencies: BTreeMap<String, Vec<String>>,
}

impl RustTargetHasher {
    pub fn new(rust_version: Option<String>) -> Self {
        RustTargetHasher {
            rust_version: rust_version.unwrap_or_else(|| "unknown".into()),
            ..RustTargetHasher::default()
        }
    }
}

impl Hasher for RustTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.rust_version.as_bytes());

        for versions in self.locked_dependencies.values() {
            for version in versions {
                sha.update(version.as_bytes());
            }
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
