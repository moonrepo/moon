use moon_common::path::RelativePathBuf;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct GitModule {
    pub branch: Option<String>,

    /// Absolute path to where the submodule is checked out to within the repository.
    pub checkout_dir: PathBuf,

    /// Absolute path to the submodule's `.git` directory, which is housed in the
    /// parent's `.git/modules`.
    pub git_dir: PathBuf,

    /// Relative path to the submodule checkout, defined in `.gitmodules`.
    pub path: RelativePathBuf,

    /// URL of the repository.
    pub url: String,
}

impl GitModule {
    pub fn is_root(&self) -> bool {
        self.path.as_str().is_empty()
    }
}

// https://git-scm.com/docs/gitmodules
pub fn parse_gitmodules_file(
    gitmodules_path: &Path,
    repository_root: &Path,
) -> miette::Result<FxHashMap<String, GitModule>> {
    let mut modules = FxHashMap::default();
    let mut current_module_name = None;
    let mut current_module = GitModule::default();
    let contents = fs::read_file(gitmodules_path)?;

    fn clean_line(line: &str) -> String {
        line.replace("=", "").replace("\"", "").trim().to_owned()
    }

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with("[submodule") {
            if current_module_name.is_some() {
                modules.insert(current_module_name.take().unwrap(), current_module);
                current_module = GitModule::default();
            }

            let name = line
                .replace("[submodule", "")
                .replace("\"", "")
                .replace("]", "")
                .trim()
                .to_owned();

            current_module_name = Some(name);
        } else if let Some(value) = line.strip_prefix("branch") {
            current_module.branch = Some(clean_line(value));
        } else if let Some(value) = line.strip_prefix("path") {
            current_module.path = RelativePathBuf::from(clean_line(value));
        } else if let Some(value) = line.strip_prefix("url") {
            current_module.url = clean_line(value);
        }
    }

    if current_module_name.is_some() {
        modules.insert(current_module_name.take().unwrap(), current_module);
    }

    Ok(modules
        .into_iter()
        .filter_map(|(name, mut module)| {
            let rel_path = module.path.as_str();

            if rel_path.is_empty() {
                None
            } else {
                module.checkout_dir = repository_root.join(rel_path);
                module.git_dir = repository_root.join(".git/modules").join(rel_path);

                Some((name, module))
            }
        })
        .collect())
}
