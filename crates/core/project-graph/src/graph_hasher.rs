use moon_config::{ProjectsAliasesMap, ProjectsSourcesMap};
use moon_hasher::{hash_hmap, Hasher, Sha256};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphHasher {
    aliases: ProjectsAliasesMap,

    sources: ProjectsSourcesMap,
}

impl GraphHasher {
    pub fn hash_aliases(&mut self, aliases: &ProjectsAliasesMap) {
        self.aliases.extend(aliases.to_owned());
    }

    pub fn hash_sources(&mut self, sources: &ProjectsSourcesMap) {
        self.sources.extend(sources.to_owned());
    }
}

impl Hasher for GraphHasher {
    fn hash(&self, sha: &mut Sha256) {
        hash_hmap(&self.aliases, sha);
        hash_hmap(&self.sources, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
