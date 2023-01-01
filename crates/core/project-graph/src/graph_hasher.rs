use moon_config::{ProjectsAliasesMap, ProjectsSourcesMap};
use moon_hasher::{hash_btree, Hasher, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphHasher {
    aliases: BTreeMap<String, String>,

    configs: BTreeMap<String, String>,

    sources: BTreeMap<String, String>,
}

impl GraphHasher {
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
        hash_btree(&self.sources, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
