mod inherited_tasks_config;
mod language_platform;
mod portable_path;
mod project;
mod project_config;
mod template;
mod template_config;
mod toolchain;
mod toolchain_config;
mod types;
mod validate;
mod workspace;
mod workspace_config;

pub use inherited_tasks_config::*;
pub use language_platform::*;
pub use portable_path::*;
pub use project::*;
pub use project_config::*;
pub use schematic::{Config, ConfigEnum, ConfigError, PartialConfig};
pub use template::*;
pub use template_config::*;
pub use toolchain::*;
pub use toolchain_config::*;
pub use types::*;
pub use workspace::*;
pub use workspace_config::*;

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

pub fn load_toolchain_rust_config_template() -> &'static str {
    include_str!("../templates/toolchain_rust.yml")
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
