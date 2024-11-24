use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct GitSubmodule {
    pub branch: Option<String>,
    pub path: String,
    pub url: String,
}

pub fn parse_gitmodules_file(path: &Path) -> miette::Result<FxHashMap<String, GitSubmodule>> {
    let mut modules = FxHashMap::default();
    let mut current_module_name = None;
    let mut current_module = GitSubmodule::default();
    let contents = fs::read_file(path)?;

    fn clean_line(line: &str) -> String {
        line.replace("=", "").replace("\"", "").trim().to_owned()
    }

    for line in contents.lines() {
        let line = line.trim();

        if line.starts_with("[submodule") {
            if current_module_name.is_some() {
                modules.insert(current_module_name.take().unwrap(), current_module);
                current_module = GitSubmodule::default();
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
            current_module.path = clean_line(value);
        } else if let Some(value) = line.strip_prefix("url") {
            current_module.url = clean_line(value);
        }
    }

    if current_module_name.is_some() {
        modules.insert(current_module_name.take().unwrap(), current_module);
    }

    Ok(modules
        .into_iter()
        .filter(|(_, module)| !module.path.is_empty())
        .collect())
}
