use moon_config::ProjectsSourcesMap;
use moon_hasher::{hash_hmap, Hasher, Sha256};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphHasher {
    projects: ProjectsSourcesMap,
}

impl GraphHasher {
    pub fn hash_sources(&mut self, sources: &ProjectsSourcesMap) {
        self.projects.extend(sources.to_owned());
    }
}

impl Hasher for GraphHasher {
    fn hash(&self, sha: &mut Sha256) {
        hash_hmap(&self.projects, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
