use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct YarnInfoDependency {
    pub descriptor: String,
    pub locator: String,

    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub struct YarnInfoChildren {
    #[serde(rename = "Dependencies")]
    pub dependencies: Option<Vec<YarnInfoDependency>>,

    #[serde(rename = "Exported Binaries")]
    pub exported_binaries: Option<Vec<String>>,

    #[serde(rename = "Instances")]
    pub instances: Option<i32>,

    #[serde(rename = "Version")]
    pub version: String,

    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub struct YarnInfoItem {
    pub children: YarnInfoChildren,

    pub value: String,
}

// Values are in the format of `<pkg>@<version>` or `<pkg>@<locator>`, and both
// the package name and locator may contain "@", which makes this complicated.
// However, the value we need to split on is always the 2nd "@".
fn extract_package_name(value: &str) -> String {
    // Slice the string and remove the first char incase its an npm scope
    let name = &value[1..];

    // Then find the next @ and slice up until it
    if let Some(at_index) = name.find('@') {
        return value[0..at_index].to_owned();
    }

    // Unknown, so just use the whole thing
    value.to_owned()
}

// `yarn info` is a stream of JSON objects, so they need to be parsed separately
// and combined into a new result.
pub fn parse_yarn_info<T: AsRef<str>>(
    json: T,
) -> Result<HashMap<String, String>, serde_json::Error> {
    let mut deps = HashMap::new();

    for item in json.as_ref().split('\n') {
        let data: YarnInfoItem = serde_json::from_str(item)?;

        deps.insert(extract_package_name(&data.value), data.children.version);
    }

    Ok(deps)
}
