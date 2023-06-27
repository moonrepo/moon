use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_common::Id;
use moon_logger::trace;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectsState {
    pub globs: Vec<String>,

    pub last_hash: String,

    pub last_glob_time: u128,

    pub projects: FxHashMap<Id, String>,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ProjectsState);
