use moon_hasher::{Digest, Hasher, Sha256};
use moon_rust_lang::cargo_toml::DependencyDetail;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RustManifestHasher {
    pub dependencies: BTreeMap<String, DependencyDetail>,
    pub name: String,
}

impl Hasher for RustManifestHasher {
    fn hash(&self, sha: &mut Sha256) {
        sha.update(self.name.as_bytes());

        for (name, dep) in &self.dependencies {
            sha.update(name.as_bytes());

            if let Some(registry) = &dep.registry {
                sha.update(registry.as_bytes());
            }

            if let Some(registry_index) = &dep.registry_index {
                sha.update(registry_index.as_bytes());
            }

            if let Some(path) = &dep.path {
                sha.update(path.as_bytes());
            }

            if let Some(git) = &dep.git {
                sha.update(git.as_bytes());
            }

            if let Some(branch) = &dep.branch {
                sha.update(branch.as_bytes());
            }

            if let Some(tag) = &dep.tag {
                sha.update(tag.as_bytes());
            }

            if let Some(rev) = &dep.rev {
                sha.update(rev.as_bytes());
            }

            for feature in &dep.features {
                sha.update(feature.as_bytes());
            }

            sha.update(dep.default_features.to_string().as_bytes());
            sha.update(dep.inherited.to_string().as_bytes());
            sha.update(dep.optional.to_string().as_bytes());
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
