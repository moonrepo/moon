use moon_hash::hash_content;
use std::env::consts;

hash_content!(
    pub struct SystemTargetHash<'proc> {
        // Architecture
        arch: &'proc str,

        // Operating system
        os: &'proc str,
    }
);

impl SystemTargetHash<'_> {
    pub fn new() -> Self {
        SystemTargetHash {
            arch: consts::ARCH,
            os: consts::OS,
        }
    }
}
