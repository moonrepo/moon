use moon_hash::hash_content;
use moon_lang::LockfileDependencyVersions;
use moon_node_lang::PackageJson;
use std::collections::BTreeMap;

hash_content!(
    pub struct BunTargetHash {
        // Bun version
        bun_version: String,

        // All the dependencies of the project (including dev and peer),
        // and the hashes corresponding with their versions
        dependencies: BTreeMap<String, Vec<String>>,
    }
);

impl BunTargetHash {
    pub fn new(bun_version: Option<String>) -> Self {
        BunTargetHash {
            bun_version: bun_version.unwrap_or_else(|| "unknown".into()),
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
