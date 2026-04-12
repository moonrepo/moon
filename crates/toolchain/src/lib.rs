mod spec;

use indexmap::IndexSet;
use moon_common::{Id, IdExt};
use moon_config::ToolchainsConfig;
use moon_env_var::{GlobalEnvBag, as_bool};

pub use moon_config::{UnresolvedVersionSpec, VersionSpec};
pub use spec::*;

pub fn requires_proto(bag: &GlobalEnvBag, config: &ToolchainsConfig) -> bool {
    let Some(force_globals) = bag.get("MOON_TOOLCHAIN_FORCE_GLOBALS") else {
        return config.requires_proto();
    };

    if force_globals.is_empty() {
        return config.requires_proto();
    }

    if check_enabled(&force_globals) {
        return false;
    }

    let mut count = 0;
    let force_globals = force_globals.split(',').collect::<Vec<_>>();

    for id in config.plugins.keys() {
        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        if force_globals.contains(&stable_id.as_str())
            || force_globals.contains(&unstable_id.as_str())
        {
            continue;
        } else {
            count += 1;
        }
    }

    count > 0
}

pub fn is_using_global_toolchains(bag: &GlobalEnvBag) -> bool {
    bag.get_as("MOON_TOOLCHAIN_FORCE_GLOBALS", as_bool)
        .unwrap_or_default()
}

pub fn is_using_global_toolchain(bag: &GlobalEnvBag, id: impl AsRef<str>) -> bool {
    let (stable_id, unstable_id) = Id::stable_and_unstable(id);

    bag.get("MOON_TOOLCHAIN_FORCE_GLOBALS")
        .is_some_and(|value| {
            if check_enabled(&value) || value == stable_id.as_str() || value == unstable_id.as_str()
            {
                true
            } else if value.contains(",") {
                value
                    .split(',')
                    .any(|val| val == stable_id.as_str() || val == unstable_id.as_str())
            } else {
                false
            }
        })
}

fn check_enabled(value: &str) -> bool {
    value == "1" || value == "true" || value == "on" || value == "*"
}

pub fn get_version_env_key(id: impl AsRef<str>) -> String {
    format!(
        "PROTO_{}_VERSION",
        Id::stable(id).as_str().to_uppercase().replace('-', "_")
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

pub fn filter_and_resolve_toolchain_ids(
    enabled_list: &[Id],
    in_list: Vec<Id>,
    fallback_system: bool,
) -> Vec<Id> {
    let mut out_list = IndexSet::<Id>::default();

    for id in in_list {
        if id == "system" {
            out_list.insert(id);
            continue;
        }

        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        if enabled_list.contains(&unstable_id) {
            out_list.insert(unstable_id);
        } else if enabled_list.contains(&stable_id) {
            out_list.insert(stable_id);
        }
    }

    // And always have something
    if out_list.is_empty() && fallback_system {
        out_list.insert(Id::raw("system"));
    }

    out_list.into_iter().collect()
}
