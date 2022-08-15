use moon_hasher::{Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};

// TODO: track operating system?
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTargetHasher {
    // Version of our hasher
    #[allow(dead_code)]
    version: String,
}

impl SystemTargetHasher {
    pub fn new() -> Self {
        SystemTargetHasher {
            version: String::from("1"),
        }
    }
}

impl Hasher for SystemTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.version.as_bytes());
    }
}
