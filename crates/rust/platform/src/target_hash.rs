use moon_hash::content_hashable;
use std::collections::BTreeMap;

content_hashable!(
    pub struct RustTargetHash {
        pub rust_version: String,
        pub locked_dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl RustTargetHash {
    pub fn new(rust_version: Option<String>) -> Self {
        RustTargetHash {
            rust_version: rust_version.unwrap_or_else(|| "unknown".into()),
            locked_dependencies: BTreeMap::new(),
        }
    }
}
