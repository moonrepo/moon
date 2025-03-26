use moon_config::VcsManager;
use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct HooksHash<'cfg> {
        pub files_exist: bool,
        pub hooks: BTreeMap<&'cfg str, &'cfg [String]>,
        pub vcs: &'cfg VcsManager,
    }
);

impl<'cfg> HooksHash<'cfg> {
    pub fn new(vcs: &VcsManager) -> HooksHash {
        HooksHash {
            files_exist: false,
            hooks: BTreeMap::new(),
            vcs,
        }
    }

    pub fn add_hook(&mut self, name: &'cfg str, commands: &'cfg [String]) {
        self.hooks.insert(name, commands);
    }
}
