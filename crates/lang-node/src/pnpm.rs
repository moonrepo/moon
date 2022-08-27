use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct PnpmListDependency {
    pub description: Option<String>,
    pub from: String,
    pub resolved: String,
    pub version: String,
}

#[derive(Deserialize, Serialize)]
pub struct PnpmListItem {
    pub dependencies: Option<HashMap<String, PnpmListDependency>>,
    pub name: String,
    pub path: String,
    pub version: String,
}

pub fn parse_pnpm_list<T: AsRef<str>>(
    json: T,
) -> Result<HashMap<String, String>, serde_json::Error> {
    let mut deps = HashMap::new();
    let data: Vec<PnpmListItem> = serde_json::from_str(json.as_ref())?;

    // This is the package at the defined path
    for package in data {
        if let Some(dependencies) = &package.dependencies {
            // These are all its dependencies
            for (dependency, metadata) in dependencies {
                deps.insert(dependency.to_owned(), metadata.version.to_owned());
            }
        }
    }

    Ok(deps)
}
