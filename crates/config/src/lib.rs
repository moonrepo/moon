#![allow(clippy::disallowed_types)] // schematic

#[cfg(feature = "loader")]
mod config_cache;
mod config_finder;
#[cfg(feature = "loader")]
mod config_loader;
mod extensions_config;
#[cfg(feature = "loader")]
mod formats;
mod inherited_tasks_config;
#[cfg(feature = "loader")]
mod inherited_tasks_manager;
mod macros;
pub mod patterns;
#[cfg(feature = "proto")]
mod plugin_compat;
mod project;
mod project_config;
mod shapes;
mod task_config;
mod task_options_config;
mod template;
mod template_config;
pub mod test_utils;
mod toolchain;
mod toolchains_config;
mod workspace;
mod workspace_config;

pub use config_finder::*;
#[cfg(feature = "loader")]
pub use config_loader::*;
pub use extensions_config::*;
pub use inherited_tasks_config::*;
#[cfg(feature = "loader")]
pub use inherited_tasks_manager::*;
pub use project::*;
pub use project_config::*;
pub use schematic;
pub use semver::{Version, VersionReq};
pub use shapes::*;
pub use task_config::*;
pub use task_options_config::*;
pub use template::*;
pub use template_config::*;
pub use toolchain::*;
pub use toolchains_config::*;
pub use version_spec::{CalVer, SemVer, UnresolvedVersionSpec, VersionSpec};
pub use workspace::*;
pub use workspace_config::*;

use schematic::{Config, PartialConfig};

pub fn finalize_config<T: Config>(config: T::Partial) -> miette::Result<T> {
    Ok(T::from_partial(config.finalize(&Default::default())?))
}
