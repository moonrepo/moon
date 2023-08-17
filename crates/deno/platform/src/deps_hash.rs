use moon_hash::content_hashable;
use std::collections::BTreeMap;

pub type DepsMap = BTreeMap<String, String>;
pub type DepsAliasesMap = BTreeMap<String, DepsMap>;

content_hashable!(
    #[derive(Default)]
    pub struct DenoDepsHash {
        pub aliases: DepsAliasesMap,
        pub dependencies: DepsMap,
    }
);
