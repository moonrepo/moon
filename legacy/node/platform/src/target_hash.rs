use moon_hash::hash_content;
use moon_lang::LockfileDependencyVersions;
use moon_node_lang::PackageJson;
use std::collections::BTreeMap;

hash_content!(
    pub struct NodeTargetHash {
        // Node.js version
        node_version: String,

        // All the dependencies of the project (including dev and peer),
        // and the hashes corresponding with their versions
        dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl NodeTargetHash {
    pub fn new(node_version: Option<String>) -> Self {
        NodeTargetHash {
            node_version: node_version.unwrap_or_else(|| "unknown".into()),
            dependencies: BTreeMap::new(),
        }
    }

    /// Hash `package.json` dependencies as version changes should bust the cache.
    pub fn hash_package_json(
        &mut self,
        package: &PackageJson,
        resolved_deps: &LockfileDependencyVersions,
    ) {
        let copy_deps = |deps: &BTreeMap<String, String>,
                         hashed: &mut BTreeMap<String, Vec<String>>| {
            for (name, version_range) in deps {
                if let Some(resolved_versions) = resolved_deps.get(name) {
                    let mut sorted_deps = resolved_versions.to_owned().clone();
                    sorted_deps.sort();
                    hashed.insert(name.to_owned(), sorted_deps);
                } else {
                    // No match, just use the range itself
                    hashed.insert(name.to_owned(), vec![version_range.to_owned()]);
                }
            }
        };

        if let Some(optional_deps) = &package.optional_dependencies {
            copy_deps(optional_deps, &mut self.dependencies);
        }

        if let Some(peer_deps) = &package.peer_dependencies {
            copy_deps(peer_deps, &mut self.dependencies);
        }

        if let Some(dev_deps) = &package.dev_dependencies {
            copy_deps(dev_deps, &mut self.dependencies);
        }

        if let Some(deps) = &package.dependencies {
            copy_deps(deps, &mut self.dependencies);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_hash::ContentHasher;
    use moon_node_lang::PackageJsonCache;
    use rustc_hash::FxHashMap;

    fn to_hash(content: &NodeTargetHash) -> String {
        let mut hasher = ContentHasher::new("Test");
        hasher.hash_content(content).unwrap();
        hasher.generate_hash().unwrap()
    }

    #[test]
    fn returns_default_hash() {
        let hasher = NodeTargetHash::new(Some("0.0.0".into()));

        assert_eq!(
            to_hash(&hasher),
            "6c2b8e2e909d85e4c20044bc8a8d542e6c8f39bcdf59d09c17791b8176e028ba"
        );
    }

    #[test]
    fn returns_same_hash_if_called_again() {
        let hasher = NodeTargetHash::new(Some("0.0.0".into()));

        assert_eq!(to_hash(&hasher), to_hash(&hasher));
    }

    #[test]
    fn returns_different_hash_for_diff_contents() {
        let hasher1 = NodeTargetHash::new(Some("0.0.0".into()));
        let hasher2 = NodeTargetHash::new(Some("1.0.0".into()));

        assert_ne!(to_hash(&hasher1), to_hash(&hasher2));
    }

    mod btreemap {
        use super::*;

        #[test]
        fn returns_same_hash_for_same_value_inserted() {
            let resolved_deps = FxHashMap::default();

            let mut package1 = PackageJsonCache::default();
            package1.add_dependency("react", "17.0.0", true);

            let mut hasher1 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher1.hash_package_json(&package1.data, &resolved_deps);

            let mut hasher2 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher2.hash_package_json(&package1.data, &resolved_deps);
            hasher2.hash_package_json(&package1.data, &resolved_deps);

            assert_eq!(to_hash(&hasher1), to_hash(&hasher2));
        }

        #[test]
        fn returns_same_hash_for_diff_order_insertion() {
            let resolved_deps = FxHashMap::default();

            let mut package1 = PackageJsonCache::default();
            package1.add_dependency("react", "17.0.0", true);

            let mut package2 = PackageJsonCache::default();
            package2.add_dependency("react-dom", "17.0.0", true);

            let mut hasher1 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher1.hash_package_json(&package2.data, &resolved_deps);
            hasher1.hash_package_json(&package1.data, &resolved_deps);

            let mut hasher2 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher2.hash_package_json(&package1.data, &resolved_deps);
            hasher2.hash_package_json(&package2.data, &resolved_deps);

            assert_eq!(to_hash(&hasher1), to_hash(&hasher2));
        }

        #[test]
        fn returns_diff_hash_for_overwritten_value() {
            let resolved_deps = FxHashMap::default();

            let mut package1 = PackageJsonCache::default();
            package1.add_dependency("react", "17.0.0", true);

            let mut package2 = PackageJsonCache::default();
            package2.add_dependency("react", "18.0.0", true);

            let mut hasher1 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher1.hash_package_json(&package1.data, &resolved_deps);

            let hash1 = to_hash(&hasher1);

            hasher1.hash_package_json(&package2.data, &resolved_deps);

            let hash2 = to_hash(&hasher1);

            assert_ne!(hash1, hash2);
        }
    }

    mod package_json {
        use super::*;

        #[test]
        fn supports_all_dep_types() {
            let resolved_deps = FxHashMap::default();

            let mut package = PackageJsonCache::default();
            package.add_dependency("moment", "10.0.0", true);

            let mut hasher1 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher1.hash_package_json(&package.data, &resolved_deps);
            let hash1 = to_hash(&hasher1);

            package.data.dev_dependencies =
                Some(BTreeMap::from([("eslint".to_owned(), "8.0.0".to_owned())]));

            let mut hasher2 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher2.hash_package_json(&package.data, &resolved_deps);
            let hash2 = to_hash(&hasher2);

            package.data.peer_dependencies =
                Some(BTreeMap::from([("react".to_owned(), "18.0.0".to_owned())]));

            let mut hasher3 = NodeTargetHash::new(Some("0.0.0".into()));
            hasher3.hash_package_json(&package.data, &resolved_deps);
            let hash3 = to_hash(&hasher3);

            assert_ne!(hash1, hash2);
            assert_ne!(hash1, hash3);
            assert_ne!(hash2, hash3);
        }

        #[test]
        fn uses_version_from_resolved_deps() {
            let resolved_deps =
                FxHashMap::from_iter([("prettier".to_owned(), vec!["2.1.3".to_owned()])]);

            let mut package = PackageJsonCache::default();
            package.add_dependency("prettier", "^2.0.0", true);
            package.add_dependency("rollup", "^2.0.0", true);

            let mut hasher = NodeTargetHash::new(Some("0.0.0".into()));
            hasher.hash_package_json(&package.data, &resolved_deps);

            assert_eq!(
                hasher.dependencies,
                BTreeMap::from([
                    ("prettier".to_owned(), vec!["2.1.3".to_owned()]),
                    ("rollup".to_owned(), vec!["^2.0.0".to_owned()])
                ])
            )
        }

        #[test]
        fn sorts_versions_before_hashing_them() {
            let resolved_deps = FxHashMap::from_iter([(
                "prettier".to_owned(),
                vec!["uio".to_owned(), "abc".to_owned(), "123".to_owned()],
            )]);

            let mut package = PackageJsonCache::default();
            package.add_dependency("prettier", "^2.0.0", true);

            let mut hasher = NodeTargetHash::new(Some("0.0.0".into()));
            hasher.hash_package_json(&package.data, &resolved_deps);

            assert_eq!(
                hasher.dependencies,
                BTreeMap::from([(
                    "prettier".to_owned(),
                    vec!["123".to_owned(), "abc".to_owned(), "uio".to_owned()]
                ),])
            )
        }
    }
}
