use moon_common::path::RelativePathBuf;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct GitModule {
    pub branch: Option<String>,
    pub path: RelativePathBuf,
    pub url: String,
}

impl GitModule {
    pub fn is_root(&self) -> bool {
        self.path.as_str().is_empty()
    }
}

pub fn parse_gitmodules_file(path: &Path) -> miette::Result<FxHashMap<String, GitModule>> {
    let mut modules = FxHashMap::default();
    let mut current_module_name = None;
    let mut current_module = GitModule::default();
    let contents = fs::read_file(path)?;

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
        .filter(|(_, module)| !module.path.as_str().is_empty())
        .collect())
}
