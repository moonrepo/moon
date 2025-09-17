use moon_config::VcsManager;
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct HooksHash<'cfg> {
        pub hooks: BTreeMap<&'cfg str, &'cfg [String]>,
        pub vcs: &'cfg VcsManager,
        pub version: u8,
    }
);

impl<'cfg> HooksHash<'cfg> {
    pub fn new(vcs: &VcsManager) -> HooksHash<'_> {
        HooksHash {
            hooks: BTreeMap::new(),
            vcs,
            version: 2,
        }
    }

    pub fn add_hook(&mut self, name: &'cfg str, commands: &'cfg [String]) {
        self.hooks.insert(name, commands);
    }
}
