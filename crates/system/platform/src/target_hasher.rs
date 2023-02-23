use moon_hasher::{Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::env::consts;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTargetHasher {
    // Architecture
    arch: String,

    // Operating system
    os: String,
}

impl SystemTargetHasher {
    pub fn new() -> Self {
        SystemTargetHasher {
            arch: consts::ARCH.to_owned(),
            os: consts::OS.to_owned(),
        }
    }
}

impl Hasher for SystemTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.arch.as_bytes());
        sha.update(self.os.as_bytes());
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
