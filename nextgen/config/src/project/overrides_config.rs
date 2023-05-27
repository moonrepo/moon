use crate::validate::validate_semver;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Config, Debug, Deserialize, Serialize)]
pub struct ProjectToolchainCommonToolConfig {
    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Clone, Config, Debug, Deserialize, Serialize)]
pub struct ProjectToolchainTypeScriptConfig {
    pub disabled: bool,
    pub route_out_dir_to_cache: Option<bool>,
    pub sync_project_references: Option<bool>,
    pub sync_project_references_to_paths: Option<bool>,
}

#[derive(Clone, Config, Debug, Deserialize, Serialize)]
pub struct ProjectToolchainConfig {
    #[setting(nested)]
    pub node: Option<ProjectToolchainCommonToolConfig>,

    #[setting(nested)]
    pub rust: Option<ProjectToolchainCommonToolConfig>,

    #[setting(nested)]
    pub typescript: Option<ProjectToolchainTypeScriptConfig>,
}

impl ProjectToolchainConfig {
    pub fn is_typescript_enabled(&self) -> bool {
        self.typescript
            .as_ref()
            .map(|ts| !ts.disabled)
            .unwrap_or(true)
    }
}

#[derive(Clone, Config, Debug, Deserialize, Serialize)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    pub exclude: Vec<Id>,

    // None = Include all
    // [] = Include none
    // [...] = Specific includes
    pub include: Option<Vec<Id>>,

    pub rename: FxHashMap<Id, Id>,
}

#[derive(Clone, Config, Debug, Deserialize, Serialize)]
pub struct ProjectWorkspaceConfig {
    #[setting(nested)]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
}
