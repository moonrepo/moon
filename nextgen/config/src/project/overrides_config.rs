use crate::validate::validate_semver;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;

#[derive(Config)]
pub struct ProjectToolchainCommonToolConfig {
    #[setting(validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Config)]
pub struct ProjectToolchainTypeScriptConfig {
    pub disabled: bool,
    pub route_out_dir_to_cache: Option<bool>,
    pub sync_project_references: Option<bool>,
    pub sync_project_references_to_paths: Option<bool>,
}

#[derive(Config)]
pub struct ProjectToolchainConfig {
    #[setting(nested)]
    pub node: Option<ProjectToolchainCommonToolConfig>,

    #[setting(nested)]
    pub rust: Option<ProjectToolchainCommonToolConfig>,

    #[setting(nested)]
    pub typescript: Option<ProjectToolchainTypeScriptConfig>,
}

#[derive(Config)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    pub exclude: Vec<Id>,
    pub include: Vec<Id>,
    pub rename: FxHashMap<Id, Id>,
}

#[derive(Config)]
pub struct ProjectWorkspaceConfig {
    #[setting(nested)]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
}
