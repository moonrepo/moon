use moon_hash::hash_content;
use moon_rust_lang::cargo_toml::DependencyDetail;
use std::collections::BTreeMap;

hash_content!(
    pub struct RustManifestHash {
        pub dependencies: BTreeMap<String, DependencyDetail>,
        pub name: String,
    }
);
