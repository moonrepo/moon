use moon_config::UnresolvedVersionSpec;
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct PythonToolchainHash {
        pub version: UnresolvedVersionSpec,
        pub dependencies: BTreeMap<String, Vec<String>>,
    }
);
