use moon_hash::content_hashable;
use moon_rust_lang::cargo_toml::DependencyDetail;
use std::collections::BTreeMap;

content_hashable!(
    pub struct RustManifestHash {
        pub dependencies: BTreeMap<String, DependencyDetail>,
        pub name: String,
    }
);
