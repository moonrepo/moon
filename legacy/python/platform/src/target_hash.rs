use moon_hash::hash_content;
use std::collections::BTreeMap;

hash_content!(
    pub struct PythonTargetHash {
        pub python_version: String,
        pub locked_dependencies: BTreeMap<String, Vec<String>>,
    }
);

// impl PythonTargetHash {
//     pub fn new(python_version: Option<String>) -> Self {
//         PythonTargetHash {
//             python_version: python_version.unwrap_or_else(|| "unknown".into()),
//             locked_dependencies: BTreeMap::new(),
//         }
//     }
// }
