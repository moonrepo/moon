use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_logger::trace;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DependenciesState {
    pub last_hash: String,

    pub last_install_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(DependenciesState);
