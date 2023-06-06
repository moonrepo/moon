use moon_config::{CodeownersConfig, OwnersConfig, OwnersPaths};
use moon_hasher::{Digest, Hasher, Sha256};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeownersHasher<'cfg> {
    pub workspace: &'cfg CodeownersConfig,
    pub projects: BTreeMap<&'cfg str, &'cfg OwnersConfig>,
}

impl<'cfg> CodeownersHasher<'cfg> {
    pub fn new(workspace: &CodeownersConfig) -> CodeownersHasher {
        CodeownersHasher {
            workspace,
            projects: BTreeMap::new(),
        }
    }

    pub fn add_project(&mut self, name: &'cfg str, config: &'cfg OwnersConfig) {
        self.projects.insert(name, config);
    }
}

fn hash_paths_map(map: &FxHashMap<String, Vec<String>>, sha: &mut Sha256) {
    for (key, values) in map {
        sha.update(key.as_bytes());

        for value in values {
            sha.update(value.as_bytes());
        }
    }
}

impl<'cfg> Hasher for CodeownersHasher<'cfg> {
    fn hash(&self, sha: &mut Sha256) {
        // workspace
        hash_paths_map(&self.workspace.global_paths, sha);
        sha.update(self.workspace.order_by.to_string().as_bytes());

        // projects
        for (name, config) in &self.projects {
            sha.update(name.as_bytes());
            hash_paths_map(&config.custom_groups, sha);

            if let Some(default_owner) = &config.default_owner {
                sha.update(default_owner.as_bytes());
            }

            sha.update(config.optional.to_string().as_bytes());

            match &config.paths {
                OwnersPaths::List(list) => {
                    for path in list {
                        sha.update(path.as_bytes());
                    }
                }
                OwnersPaths::Map(map) => {
                    hash_paths_map(map, sha);
                }
            };

            sha.update(config.required_approvals.to_string().as_bytes());
        }
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
