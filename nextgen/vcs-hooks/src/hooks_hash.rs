use moon_config::VcsManager;
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct HooksHash<'cfg> {
        hooks: BTreeMap<&'cfg str, &'cfg [String]>,
        vcs: &'cfg VcsManager,
    }
);

impl<'cfg> HooksHash<'cfg> {
    pub fn new(vcs: &VcsManager) -> HooksHash {
        HooksHash {
            hooks: BTreeMap::new(),
            vcs,
        }
    }

    pub fn add_hook(&mut self, name: &'cfg str, commands: &'cfg [String]) {
        self.hooks.insert(name, commands);
    }
}
