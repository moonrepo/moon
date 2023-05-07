use moon_hasher::{hash_vec, Hasher, Sha256};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBinsHasher {
    pub bins: Vec<String>,
}

impl Hasher for RustBinsHasher {
    fn hash(&self, sha: &mut Sha256) {
        hash_vec(&self.bins, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
