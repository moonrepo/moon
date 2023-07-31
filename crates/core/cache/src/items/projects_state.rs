use crate::cache_item;
use crate::helpers::get_cache_mode;
use moon_common::path::WorkspaceRelativePathBuf;
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
    pub last_hash: String,

    pub projects: FxHashMap<Id, WorkspaceRelativePathBuf>,

    #[serde(skip)]
    pub path: PathBuf,
}

cache_item!(ProjectsState);
