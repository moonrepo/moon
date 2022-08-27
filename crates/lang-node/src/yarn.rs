use serde_json::Value;
use std::collections::HashMap;

// `yarn info` is a stream of JSON objects or strings, so they need to be parsed separately
// and combined into a new result.
pub fn parse_yarn_info<T: AsRef<str>>(
    json: T,
) -> Result<HashMap<String, String>, serde_json::Error> {
    let mut deps = HashMap::new();

    let mut add_dep = |item: &str| {
        if let Some(at_index) = item.rfind('@') {
            deps.insert(
                item[0..at_index].to_owned(),
                item[(at_index + 1)..].to_owned(),
            );
        }
    };

    for item in json.as_ref().split('\n') {
        let data: Value = serde_json::from_str(item)?;

        match data {
            Value::String(item) => {
                add_dep(&item);
            }
            Value::Object(map) => {
                if let Some(Value::String(item)) = map.get("value") {
                    add_dep(item);
                }
            }
            _ => {}
        }
    }

    Ok(deps)
}
