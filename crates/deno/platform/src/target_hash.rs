use moon_hash::content_hashable;
use std::collections::BTreeMap;

content_hashable!(
    pub struct DenoTargetHash {
        // Deno version
        deno_version: String,

        // All the dependencies (and their integrity hashes) of the project
        dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl DenoTargetHash {
    pub fn new(deno_version: Option<String>) -> Self {
        DenoTargetHash {
            deno_version: deno_version.unwrap_or_else(|| "unknown".into()),
            dependencies: BTreeMap::new(),
        }
    }

    pub fn hash_deps(&mut self, dependencies: BTreeMap<String, Vec<String>>) {
        self.dependencies = dependencies;
    }
}
