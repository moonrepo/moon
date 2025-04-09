#![allow(clippy::disallowed_types)] // schematic

#[cfg(feature = "loader")]
mod config_cache;
mod config_finder;
#[cfg(feature = "loader")]
mod config_loader;
mod inherited_tasks_config;
mod language_platform;
mod macros;
pub mod patterns;
mod portable_path;
mod project;
mod project_config;
mod shapes;
mod template;
mod template_config;
#[cfg(feature = "template")]
mod templates;
mod toolchain;
mod toolchain_config;
mod types;
mod validate;
mod workspace;
mod workspace_config;

pub use config_finder::*;
#[cfg(feature = "loader")]
pub use config_loader::*;
pub use inherited_tasks_config::*;
pub use language_platform::*;
pub use portable_path::*;
pub use project::*;
pub use project_config::*;
pub use schematic;
pub use semver::{Version, VersionReq};
pub use shapes::*;
pub use template::*;
pub use template_config::*;
#[cfg(feature = "template")]
pub use templates::*;
pub use toolchain::*;
pub use toolchain_config::*;
pub use types::*;
pub use version_spec::{CalVer, SemVer, UnresolvedVersionSpec, VersionSpec};
pub use workspace::*;
pub use workspace_config::*;

use schematic::{Config, PartialConfig};

pub fn finalize_config<T: Config>(config: T::Partial) -> miette::Result<T> {
    Ok(T::from_partial(config.finalize(&Default::default())?))
}
