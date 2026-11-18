#![allow(clippy::disallowed_types)] // schematic

mod extensions_config;
mod inherited_tasks_config;
mod inherited_tasks_manager;
mod macros;
mod merging;
pub mod patterns;
mod project;
mod project_config;
mod shapes;
mod task;
mod task_config;
mod task_options_config;
mod template;
mod template_config;
pub mod test_utils;
mod toolchain;
mod toolchains_config;
mod workspace;
mod workspace_config;

pub use extensions_config::*;
pub use inherited_tasks_config::*;
pub use inherited_tasks_manager::*;
pub use merging::*;
pub use project::*;
pub use project_config::*;
pub use schematic;
pub use shapes::*;
pub use task::*;
pub use task_config::*;
pub use task_options_config::*;
pub use template::*;
pub use template_config::*;
pub use toolchain::*;
pub use toolchains_config::*;
pub use version_spec::{Range, Requirement, UnresolvedVersionSpec, Version, VersionSpec};
pub use workspace::*;
pub use workspace_config::*;

use schematic::{Config, ConfigError, PartialConfig};

/// Apply the default values and environment variables to a partial configuration
/// that was not loaded through a config loader, and then finalize it. The loader
/// merges these as layers of their own, so `PartialConfig::finalize` doesn't.
pub(crate) fn apply_partial_layers<P: PartialConfig>(
    partial: P,
    context: &P::Context,
) -> Result<P, ConfigError> {
    // Defaults are read against the partial, and go under it
    let mut partial = match partial.default_values(context)? {
        Some(mut defaults) => {
            defaults.merge(context, partial)?;
            defaults
        }
        None => partial,
    };

    // Variables are read against the result, and go over it
    if let Some(env) = partial.env_values()? {
        partial.merge(context, env)?;
    }

    partial.finalize(context)
}

pub fn finalize_config<T: Config>(config: T::Partial) -> miette::Result<T> {
    Ok(T::from_partial(apply_partial_layers(
        config,
        &Default::default(),
    )?))
}
