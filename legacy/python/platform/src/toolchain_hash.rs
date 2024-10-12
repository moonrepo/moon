use moon_config::{PipConfig, UnresolvedVersionSpec};
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct PythonToolchainHash {
        pub pip: Option<PipConfig>,
        pub version: Option<UnresolvedVersionSpec>,
        pub requirements_dependencies: String,
    }
);

impl PythonToolchainHash {
    pub fn new(python_version: Option<UnresolvedVersionSpec>, pip_config: Option<PipConfig>) -> Self {
        PythonToolchainHash {
            version: python_version,
            requirements_dependencies: "".into(),
            pip: pip_config,
        }
    }
}