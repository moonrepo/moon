use crate::portable_path::PortablePath;
use crate::project::TaskConfig;
use moon_common::Id;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

pub type FileGroupsMap = FxHashMap<Id, Vec<PortablePath>>;

pub type ProjectsSourcesMap = FxHashMap<Id, String>;

pub type ProjectsAliasesMap = FxHashMap<String, Id>;

pub type TasksConfigsMap = BTreeMap<Id, TaskConfig>;

#[macro_export]
macro_rules! cacheable {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        $impl
    };
}

#[macro_export]
macro_rules! cacheable_enum {
    ($impl:item) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(rename_all = "kebab-case")]
        $impl
    };
}
