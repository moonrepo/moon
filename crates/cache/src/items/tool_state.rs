use crate::cache_item;
use crate::helpers::{is_readable, is_writable};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, time};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolState {
    #[serde(default)]
    pub last_version_check_time: u128,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ToolState);
