use moon_hash::content_hashable;
use std::env::consts;

content_hashable!(
    pub struct SystemTargetHash {
        // Architecture
        arch: String,

        // Operating system
        os: String,
    }
);

impl SystemTargetHash {
    pub fn new() -> Self {
        SystemTargetHash {
            arch: consts::ARCH.to_owned(),
            os: consts::OS.to_owned(),
        }
    }
}
