use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, json, time};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DependenciesState {
    pub last_install_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(DependenciesState);
