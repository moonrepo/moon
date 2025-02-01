use cached::proc_macro::cached;
use moon_lang::LockfileDependencyVersions;
use pep508_rs::{Requirement, VerbatimUrl};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;
use std::str::FromStr;

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();
    let file = fs::open_file(&path)?;

    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(parsed) = Requirement::<VerbatimUrl>::from_str(&line) {
            deps.entry(parsed.name.to_string())
                .or_default()
                .push(line.clone());
        }
    }

    Ok(deps)
}
