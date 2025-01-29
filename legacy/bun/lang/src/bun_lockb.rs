use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;
use yarn_lock_parser::parse_str;

pub fn load_binary_lockfile_dependencies(
    lockfile_text: Arc<String>,
    path: PathBuf,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    // Bun lockfiles are binary, but can be represented as text in Yarn v1 format!
    let entries = match parse_str(&lockfile_text) {
        Ok(data) => data.entries,
        Err(_) => {
            warn!(
                lockfile = ?path,
                "Failed to parse bun.lockb (in Yarn format). Task generated hashes will be different.",
            );

            return Ok(deps);
        }
    };

    for entry in entries {
        // All workspace dependencies have empty integrities, so we will skip them
        if entry.integrity.is_empty() {
            continue;
        }

        let dep = deps.entry(entry.name.to_owned()).or_default();
        dep.push(entry.integrity.to_owned());
    }

    Ok(deps)
}
