use moon_config::{ProjectsAliasesMap, ProjectsSourcesMap};
use moon_hasher::{hash_btree, Digest, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphHasher {
    aliases: BTreeMap<String, String>,

    configs: BTreeMap<String, String>,

    sources: BTreeMap<String, String>,

    // Version of the moon CLI. We need to include this so that the graph
    // cache is invalidated between each release, otherwise internal Rust
    // changes (in project or task crates) are not reflected until the cache
    // is invalidated, which puts the program in a weird state.
    version: String,
}

impl GraphHasher {
    pub fn new() -> Self {
        GraphHasher {
            aliases: BTreeMap::default(),
            configs: BTreeMap::default(),
            sources: BTreeMap::default(),
            version: env::var("MOON_VERSION").unwrap_or_default(),
        }
    }

    pub fn hash_aliases(&mut self, aliases: &ProjectsAliasesMap) {
        self.aliases.extend(aliases.to_owned());
    }

    pub fn hash_configs(&mut self, configs: &BTreeMap<String, String>) {
        self.configs.extend(configs.to_owned());
    }

    pub fn hash_sources(&mut self, sources: &ProjectsSourcesMap) {
        self.sources.extend(sources.to_owned());
    }
}

impl Hasher for GraphHasher {
    fn hash(&self, sha: &mut Sha256) {
        hash_btree(&self.aliases, sha);
        hash_btree(&self.configs, sha);
        hash_btree(&self.sources, sha);
        sha.update(self.version.as_bytes());
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
