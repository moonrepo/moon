use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use yarn_lock_parser::{parse_str, Entry};

#[cached(result)]
pub fn load_lockfile_dependencies(
    lockfile_text: String,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    // Lockfile barfs on the Bun comments: https://github.com/robertohuertasm/yarn-lock-parser/issues/15
    let mut lockfile_text = lockfile_text
        .lines()
        .filter(|line| !line.starts_with("# bun"))
        .collect::<Vec<_>>()
        .join("\n");

    lockfile_text.push_str("\n");

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
