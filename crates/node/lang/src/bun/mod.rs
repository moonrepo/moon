use cached::proc_macro::cached;
use miette::IntoDiagnostic;
use moon_lang::LockfileDependencyVersions;
use moon_utils::regex;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use yarn_lock_parser::{parse_str, Entry};

static REPLACE_WS_VERSION: Lazy<regex::Regex> =
    Lazy::new(|| regex::create_regex("version \"workspace:([^\"]+)\"").unwrap());

#[cached(result)]
pub fn load_lockfile_dependencies(
    lockfile_text: String,
) -> miette::Result<LockfileDependencyVersions> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    // Lockfile explodes: https://github.com/robertohuertasm/yarn-lock-parser/issues/15
    let mut lockfile_text = lockfile_text
        .lines()
        .filter_map(|line| {
            if line.starts_with("# bun") {
                None
            } else if line.contains("version \"workspace:") {
                Some(
                    REPLACE_WS_VERSION
                        .replace(line, "version \"0.0.0\"")
                        .to_string(),
                )
            } else {
                Some(line.to_owned())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    lockfile_text.push('\n');

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
