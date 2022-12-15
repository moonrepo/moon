use crate::PNPM;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::config_cache;
use moon_utils::yaml::read as read_yaml;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

config_cache!(PnpmWorkspace, PNPM.config_files[2], read_yaml);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmWorkspace {
    pub packages: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}
