use moon_config::VcsClient;
use moon_hash::hash_fingerprint;
use std::collections::BTreeMap;

hash_fingerprint!(
    pub struct HooksFingerprint<'cfg> {
        pub hooks: BTreeMap<&'cfg str, &'cfg [String]>,
        pub vcs: &'cfg VcsClient,
        pub version: u8,
    }
);

impl<'cfg> HooksFingerprint<'cfg> {
    pub fn new(vcs: &VcsClient) -> HooksFingerprint<'_> {
        HooksFingerprint {
            hooks: BTreeMap::new(),
            vcs,
            version: 2,
        }
    }

    pub fn add_hook(&mut self, name: &'cfg str, commands: &'cfg [String]) {
        self.hooks.insert(name, commands);
    }
}
