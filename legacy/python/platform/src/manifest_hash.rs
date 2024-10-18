use moon_hash::hash_content;
// use moon_python_lang::pipfile::DependencyDetail;
// use std::collections::BTreeMap;

hash_content!(
    pub struct PythonManifestHash {
        // pub dependencies: BTreeMap<String, DependencyDetail>,
        pub name: String,
    }
);
