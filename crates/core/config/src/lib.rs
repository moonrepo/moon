mod errors;
mod helpers;
mod project;
mod template;
mod toolchain;
mod types;
mod validators;
mod workspace;

pub use errors::{
    format_error_line, format_figment_errors, map_validation_errors_to_figment_errors, ConfigError,
};
pub use project::*;
pub use template::*;
pub use toolchain::*;
pub use types::*;
pub use validator::ValidationErrors;
pub use workspace::*;

pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_workspace_node_config_template() -> &'static str {
    include_str!("../templates/workspace_node.yml")
}

pub fn load_workspace_typescript_config_template() -> &'static str {
    include_str!("../templates/workspace_typescript.yml")
}

pub fn load_global_project_config_template() -> &'static str {
    include_str!("../templates/global_project.yml")
}

// pub fn load_project_config_template() -> &'static str {
//     include_str!("../templates/project.yml")
// }

pub fn load_template_config_template() -> &'static str {
    include_str!("../templates/template.yml")
}
