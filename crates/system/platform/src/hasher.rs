use moon_hasher::{Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::env::consts;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTargetHasher {
    // Architecture
    arch: String,

    // Operating system
    os: String,

    // Version of our hasher
    #[allow(dead_code)]
    version: String,
}

impl SystemTargetHasher {
    pub fn new() -> Self {
        SystemTargetHasher {
            arch: consts::ARCH.to_owned(),
            os: consts::OS.to_owned(),
            version: "1".into(),
        }
    }
}

impl Hasher for SystemTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.version.as_bytes());
        sha.update(self.arch.as_bytes());
        sha.update(self.os.as_bytes());
    }
}
