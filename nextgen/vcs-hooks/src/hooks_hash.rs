use moon_config::VcsManager;
use moon_hash::content_hashable;
use std::collections::BTreeMap;

content_hashable!(
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
