pub mod detect;
mod runtime;
mod spec;

pub use runtime::*;
pub use spec::*;

use moon_env_var::{GlobalEnvBag, as_bool};

pub fn is_using_global_toolchains() -> bool {
    GlobalEnvBag::instance()
        .get_as("MOON_TOOLCHAIN_FORCE_GLOBALS", as_bool)
        .unwrap_or_default()
}

pub fn is_using_global_toolchain(id: impl AsRef<str>) -> bool {
    let id = id.as_ref();

    GlobalEnvBag::instance()
        .get("MOON_TOOLCHAIN_FORCE_GLOBALS")
        .is_some_and(|value| {
            if value == "1" || value == "true" || value == "on" || value == id {
                true
            } else if value.contains(",") {
                value.split(',').any(|val| val == id)
            } else {
                false
            }
        })
}

pub fn get_version_env_key(id: impl AsRef<str>) -> String {
    format!(
        "PROTO_{}_VERSION",
        id.as_ref()
            .to_uppercase()
            .replace('-', "_")
            .replace("UNSTABLE_", "")
    )
}

pub fn get_version_env_value(version: &UnresolvedVersionSpec) -> String {
    // If we have a "latest" alias, use "*" as a version instead,
    // otherwise latest will attempt to use a possibly uninstalled
    // version, while * will use any available/installed version.
    if version.is_latest() {
        return "*".into();
    }

    version.to_string()
}
