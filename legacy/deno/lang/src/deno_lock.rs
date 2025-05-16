use cached::proc_macro::cached;
use deno_lockfile::LockfileContent;
use miette::IntoDiagnostic;
use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use starbase_utils::json::{self, JsonValue};
use std::path::PathBuf;

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if path.exists() {
        let lockfile_content: JsonValue = json::read_file(&path)?;
        let lockfile = LockfileContent::from_json(lockfile_content).into_diagnostic()?;

        for (key, value) in lockfile.packages.jsr {
            deps.insert(format!("jsr:{key}"), vec![value.integrity]);
        }

        for (key, value) in lockfile.packages.npm {
            if let Some(integrity) = value.integrity {
                deps.insert(format!("npm:{key}"), vec![integrity]);
            }
        }
    }

    Ok(deps)
}
