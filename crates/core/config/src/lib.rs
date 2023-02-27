mod errors;
mod helpers;
mod project;
mod tasks;
mod template;
mod toolchain;
mod types;
mod validators;
mod workspace;

pub use errors::{
    format_error_line, format_figment_errors, map_validation_errors_to_figment_errors, ConfigError,
};
pub use moon_constants::*;
pub use project::*;
pub use tasks::*;
pub use template::*;
pub use toolchain::*;
pub use types::*;
pub use validator::ValidationErrors;
pub use workspace::*;

pub fn load_workspace_config_template() -> &'static str {
    include_str!("../templates/workspace.yml")
}

pub fn load_toolchain_config_template() -> &'static str {
    include_str!("../templates/toolchain.yml")
}

pub fn load_toolchain_deno_config_template() -> &'static str {
    include_str!("../templates/toolchain_deno.yml")
}

pub fn load_toolchain_node_config_template() -> &'static str {
    include_str!("../templates/toolchain_node.yml")
}

pub fn load_toolchain_typescript_config_template() -> &'static str {
    include_str!("../templates/toolchain_typescript.yml")
}

pub fn load_tasks_config_template() -> &'static str {
    include_str!("../templates/tasks.yml")
}

pub fn load_template_config_template() -> &'static str {
    include_str!("../templates/template.yml")
}
