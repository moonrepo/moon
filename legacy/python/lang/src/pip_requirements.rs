use cached::proc_macro::cached;
use moon_lang::LockfileDependencyVersions;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use std::fs;

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();
    
    let lockfile = fs::read_to_string(path.as_path()).expect("Unable to read file");
    let dep = deps.entry(path.display().to_string()).or_default();
    dep.push(lockfile);
    Ok(deps)
}
