use moon_config::{CodeownersConfig, OwnersConfig};
use moon_hash::hash_fingerprint;
use std::collections::BTreeMap;

hash_fingerprint!(
    pub struct CodeownersFingerprint<'cfg> {
        pub file_exists: bool,
        pub projects: BTreeMap<&'cfg str, &'cfg OwnersConfig>,
        pub workspace: &'cfg CodeownersConfig,
    }
);

impl<'cfg> CodeownersFingerprint<'cfg> {
    pub fn new(workspace: &CodeownersConfig) -> CodeownersFingerprint<'_> {
        CodeownersFingerprint {
            file_exists: false,
            projects: BTreeMap::new(),
            workspace,
        }
    }

    pub fn add_project(&mut self, name: &'cfg str, config: &'cfg OwnersConfig) {
        self.projects.insert(name, config);
    }
}
