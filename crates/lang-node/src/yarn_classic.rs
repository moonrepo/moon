use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct YarnListItemActivity {
    pub id: i32,
    pub name: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct YarnListItemTreeNode {
    pub name: String,

    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub struct YarnListItemTree {
    pub trees: Vec<YarnListItemTreeNode>,

    #[serde(rename = "type")]
    pub type_of: String,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum YarnListItem {
    #[serde(rename = "activityEnd")]
    ActivityEnd { data: YarnListItemActivity },

    #[serde(rename = "activityTick")]
    ActivityTick { data: YarnListItemActivity },

    #[serde(rename = "tree")]
    Tree { data: YarnListItemTree },
}

// `yarn list` is a stream of JSON objects, so they need to be parsed separately
// and combined into a new result.
pub fn parse_yarn_list<T: AsRef<str>>(
    json: T,
) -> Result<HashMap<String, String>, serde_json::Error> {
    let mut deps = HashMap::new();

    for item in json.as_ref().split('\n') {
        let data: YarnListItem = serde_json::from_str(item)?;

        if let YarnListItem::Tree { data } = data {
            for node in data.trees {
                if let Some(at_index) = node.name.rfind('@') {
                    deps.insert(
                        node.name[0..at_index].to_owned(),
                        node.name[(at_index + 1)..].to_owned(),
                    );
                }
            }
        }
    }

    Ok(deps)
}
