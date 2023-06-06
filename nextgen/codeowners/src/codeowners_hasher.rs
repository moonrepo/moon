use moon_config::{CodeownersConfig, OwnersConfig};
use moon_hash::content_hashable;
use std::collections::BTreeMap;

content_hashable!(
    pub struct CodeownersHasher<'cfg> {
        projects: BTreeMap<&'cfg str, &'cfg OwnersConfig>,
        workspace: &'cfg CodeownersConfig,
    }
);

impl<'cfg> CodeownersHasher<'cfg> {
    pub fn new(workspace: &CodeownersConfig) -> CodeownersHasher {
        CodeownersHasher {
            projects: BTreeMap::new(),
            workspace,
        }
    }

    pub fn add_project(&mut self, name: &'cfg str, config: &'cfg OwnersConfig) {
        self.projects.insert(name, config);
    }
}
