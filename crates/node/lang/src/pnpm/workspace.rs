use cached::proc_macro::cached;
use moon_lang::config_cache;
use serde::{Deserialize, Serialize};
use starbase_utils::yaml::read_file as read_yaml;
use std::path::{Path, PathBuf};

config_cache!(PnpmWorkspace, "pnpm-workspace.yaml", read_yaml);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmWorkspace {
    pub packages: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}
