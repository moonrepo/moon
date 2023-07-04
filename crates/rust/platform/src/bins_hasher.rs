use moon_config::BinEntry;
use moon_hasher::{Digest, Hasher, Sha256};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBinsHasher {
    pub bins: Vec<BinEntry>,
}

impl Hasher for RustBinsHasher {
    fn hash(&self, sha: &mut Sha256) {
        for bin in &self.bins {
            match bin {
                BinEntry::Name(name) => {
                    sha.update(name.as_bytes());
                }
                BinEntry::Config(cfg) => {
                    sha.update(cfg.bin.as_bytes());

                    if let Some(version) = cfg.version.as_ref() {
                        sha.update(version.as_bytes());
                    }

                    sha.update(cfg.force.to_string());
                    sha.update(cfg.local.to_string());
                }
            };
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
