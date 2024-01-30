mod inherited_tasks_config;
mod language_platform;
pub mod patterns;
mod portable_path;
mod project;
mod project_config;
mod shapes;
mod template;
mod template_config;
mod toolchain;
mod toolchain_config;
mod types;
mod validate;
mod workspace;
mod workspace_config;

#[cfg(feature = "template")]
mod templates;

pub use inherited_tasks_config::*;
pub use language_platform::*;
pub use portable_path::*;
pub use project::*;
pub use project_config::*;
pub use schematic::{Config, ConfigEnum, ConfigError, PartialConfig};
pub use semver::{Version, VersionReq};
pub use shapes::*;
pub use template::*;
pub use template_config::*;
pub use toolchain::*;
pub use toolchain_config::*;
pub use types::*;
pub use version_spec::{UnresolvedVersionSpec, VersionSpec};
pub use workspace::*;
pub use workspace_config::*;

#[cfg(feature = "template")]
pub use templates::*;
