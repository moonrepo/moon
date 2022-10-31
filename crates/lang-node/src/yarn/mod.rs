use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::LockfileDependencyVersions;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use yarn_lock_parser::{parse_str, Entry};

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = HashMap::new();

    let yarn_lock_text = fs::read_to_string(&path)?;
    let entries: Vec<Entry> = parse_str(&yarn_lock_text)
        .map_err(|_| MoonError::Generic("Failed to parse lockfile".to_owned()))?;
    for entry in entries {
        let dep = deps.entry(entry.name.to_owned()).or_default();
        dep.push(entry.integrity.to_owned());
        dep.sort();
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {}
