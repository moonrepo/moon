use cached::proc_macro::cached;
use deno_lockfile::{Lockfile, NewLockfileOptions};
use miette::IntoDiagnostic;
use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::PathBuf;

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if path.exists() {
        let lockfile_content = fs::read_file(&path)?;
        let lockfile = Lockfile::new(NewLockfileOptions {
            content: &lockfile_content,
            file_path: path.clone(),
            overwrite: false,
        })
        .into_diagnostic()?;

        for (key, value) in lockfile.content.packages.jsr {
            deps.insert(format!("jsr:{key}"), vec![value.integrity]);
        }

        for (key, value) in lockfile.content.packages.npm {
            deps.insert(format!("npm:{key}"), vec![value.integrity]);
        }
    }

    Ok(deps)
}
