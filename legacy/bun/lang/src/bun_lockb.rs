use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use yarn_lock_parser::{parse_str, Entry};

#[cached(result)]
pub fn load_lockfile_dependencies(
    lockfile_text: Arc<String>,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    // Bun lockfiles are binary, but can be represented as text in Yarn v1 format!
    let entries: Vec<Entry> = parse_str(&lockfile_text).into_diagnostic()?;

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
