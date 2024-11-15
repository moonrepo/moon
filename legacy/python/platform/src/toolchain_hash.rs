use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct PythonToolchainHash {
        pub version: String,
        pub dependencies: BTreeMap<String, Vec<String>>,
    }
);
