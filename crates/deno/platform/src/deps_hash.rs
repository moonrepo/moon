use moon_hash::hash_content;
use std::collections::BTreeMap;

pub type DepsMap = BTreeMap<String, String>;
pub type DepsAliasesMap = BTreeMap<String, DepsMap>;

hash_content!(
    #[derive(Default)]
    pub struct DenoDepsHash {
        pub aliases: DepsAliasesMap,
        pub dependencies: DepsMap,
    }
);
