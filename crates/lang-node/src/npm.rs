use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct NpmListDependency {
    pub dependencies: Option<HashMap<String, NpmListDependency>>,
    pub resolved: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct NpmList {
    pub dependencies: Option<HashMap<String, NpmListDependency>>,
    pub name: String,
    pub version: Option<String>,
}

pub fn parse_npm_list<T: AsRef<str>>(
    json: T,
) -> Result<HashMap<String, String>, serde_json::Error> {
    let mut deps = HashMap::new();
    let json = json.as_ref();

    if json.is_empty() {
        return Ok(deps);
    }

    let data: NpmList = serde_json::from_str(json)?;

    if let Some(packages) = &data.dependencies {
        // This is the package at the defined path
        for package_meta in packages.values() {
            if let Some(dependencies) = &package_meta.dependencies {
                // These are all its dependencies
                for (dependency, dep_meta) in dependencies {
                    if let Some(version) = &dep_meta.version {
                        deps.insert(dependency.to_owned(), version.to_owned());
                    }
                }
            }
        }
    }

    Ok(deps)
}
