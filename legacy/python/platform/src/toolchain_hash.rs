use moon_config::{PipConfig, UnresolvedVersionSpec};
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct PythonToolchainHash {
        // pub pip: Option<PipConfig>,
        pub version: UnresolvedVersionSpec,
        pub dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl PythonToolchainHash {
    pub fn new(python_version: UnresolvedVersionSpec, pip_config: Option<PipConfig>) -> Self {
        PythonToolchainHash {
            version: python_version,
            dependencies: BTreeMap::new(),            
            // pip: pip_config,
        }
    }
}