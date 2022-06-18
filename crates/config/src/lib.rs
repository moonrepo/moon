pub mod constants;
mod errors;
pub mod package;
mod project;
mod providers;
pub mod tsconfig;
mod types;
mod validators;
mod workspace;

pub use errors::format_errors;
pub use project::global::GlobalProjectConfig;
pub use project::task::{TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType};
pub use project::{ProjectConfig, ProjectLanguage, ProjectMetadataConfig, ProjectType};
pub use types::{FileGlob, FilePath, FilePathOrGlob, ProjectID, TargetID, TaskID};
pub use validator::ValidationErrors;
pub use workspace::node::{
    default_node_version, default_npm_version, default_pnpm_version, default_yarn_version,
};
pub use workspace::{
    NodeConfig, NpmConfig, PackageManager, PnpmConfig, TypeScriptConfig, VcsConfig, VcsManager,
    WorkspaceConfig, YarnConfig,
};

pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_global_project_config_template() -> &'static str {
    include_str!("../templates/global_project.yml")
}

pub fn load_project_config_template() -> &'static str {
    include_str!("../templates/project.yml")
}
