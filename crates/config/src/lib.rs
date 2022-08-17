mod errors;
mod helpers;
mod project;
mod providers;
mod types;
mod validators;
mod workspace;

pub use errors::{format_figment_errors, ConfigError};
pub use project::dep::*;
pub use project::global::GlobalProjectConfig;
pub use project::task::*;
pub use project::task_options::*;
pub use project::*;
pub use types::*;
pub use validator::ValidationErrors;
pub use workspace::node::*;
pub use workspace::*;

pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_global_project_config_template() -> &'static str {
    include_str!("../templates/global_project.yml")
}

// pub fn load_project_config_template() -> &'static str {
//     include_str!("../templates/project.yml")
// }
