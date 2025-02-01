use crate::hash_content;
use std::collections::BTreeMap;

pub type DepsMap = BTreeMap<String, String>;
pub type DepsAliasesMap = BTreeMap<String, DepsMap>;

hash_content!(
    pub struct DepsHash<'cfg> {
        pub aliases: BTreeMap<&'cfg str, BTreeMap<&'cfg str, &'cfg str>>,
        pub dependencies: BTreeMap<&'cfg str, &'cfg str>,
        pub name: String,
    }
);

impl<'cfg> DepsHash<'cfg> {
    pub fn new(name: String) -> Self {
        DepsHash {
            aliases: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            name,
        }
    }

    pub fn add_aliases(&mut self, aliases: &'cfg BTreeMap<String, BTreeMap<String, String>>) {
        for (alias, deps) in aliases {
            let mut deps_map = BTreeMap::<&'cfg str, &'cfg str>::new();

            for (name, value) in deps {
                deps_map.insert(name, value);
            }

            self.aliases.insert(alias, deps_map);
        }
    }

    pub fn add_deps(&mut self, deps: &'cfg BTreeMap<String, String>) {
        for (name, value) in deps {
            self.dependencies.insert(name, value);
        }
    }
}
