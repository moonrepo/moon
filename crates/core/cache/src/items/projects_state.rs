use crate::cache_item;
use crate::helpers::{is_readable, is_writable};
use moon_error::MoonError;
use moon_logger::{color, trace};
use moon_utils::{fs, time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectsState {
    pub globs: Vec<String>,

    pub projects: HashMap<String, String>,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ProjectsState);
