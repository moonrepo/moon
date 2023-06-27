use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_logger::trace;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ToolState {
    pub last_versions: FxHashMap<String, String>,

    pub last_version_check_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ToolState);
