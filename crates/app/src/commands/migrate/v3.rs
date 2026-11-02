use super::utils::*;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use starbase_utils::yaml::{YamlMapping, YamlValue};
use std::path::Path;
use tracing::{instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct MigrateV3Args {
    #[arg(long, help = "Skip prompts and apply all migrations")]
    yes: bool,
}

#[instrument(skip(session))]
pub async fn v3(session: MoonSession, args: MigrateV3Args) -> SessionResult {
    if confirm_config_migration(&session, args.yes).await? {
        migrate_workspace_config_files(&session)?;
        migrate_tasks_config_files(&session)?;
        migrate_project_config_files(&session)?;
    }

    Ok(None)
}

// CONFIGURATION

fn migrate_task_options_setting(options: &mut YamlMapping, config_path: &Path, label: &str) {
    let is_enabled = |options: &YamlMapping, key: &str| {
        options
            .get(key)
            .and_then(YamlValue::as_bool)
            .is_some_and(|value| value)
    };

    // `expect` was the stricter of the two
    let on_failure = if options.contains_key("onFailure") {
        None
    } else if is_enabled(options, "expectFailure") {
        Some(("expectFailure", "expect"))
    } else if is_enabled(options, "allowFailure") {
        Some(("allowFailure", "allow"))
    } else {
        None
    };

    if let Some((old_name, value)) = on_failure {
        rename_key(options, old_name, "onFailure");

        options.insert(YamlValue::from("onFailure"), YamlValue::from(value));
    }

    for name in ["allowFailure", "expectFailure"] {
        if options
            .shift_remove(name)
            .is_some_and(|value| value.as_bool() == Some(false))
        {
            warn!(
                config = ?config_path,
                "Removed {name}: false from {label}, as onFailure has no equivalent value. If it overrode an inherited value, the inherited value now applies."
            );
        }
    }
}

fn migrate_tasks_setting(root: &mut YamlMapping, config_path: &Path) {
    if let Some(options) = root
        .get_mut("taskOptions")
        .and_then(YamlValue::as_mapping_mut)
    {
        migrate_task_options_setting(options, config_path, "taskOptions");
    }

    if let Some(tasks) = root.get_mut("tasks").and_then(YamlValue::as_mapping_mut) {
        for (id, task) in tasks.iter_mut() {
            if let Some(options) = task.get_mut("options").and_then(YamlValue::as_mapping_mut) {
                migrate_task_options_setting(
                    options,
                    config_path,
                    &format!("task {}", id.as_str().unwrap_or_default()),
                );
            }
        }
    }
}

fn migrate_workspace_setting(root: &mut YamlMapping, config_path: &Path) {
    rename_setting(
        root,
        "cache.unstable_sharedWorktreeCache",
        "cache.sharedWorktreeCache",
    );

    rename_setting(root, "generator", "codegen");

    rename_setting(root, "unstable_daemon", "daemon");

    if remove_setting(root, "experiments.asyncGraphBuilding")
        .is_some_and(|value| value.as_bool() == Some(false))
    {
        warn!(
            config = ?config_path,
            "Removed the asyncGraphBuilding experiment, as graphs are now always built asynchronously. If it was disabled to work around a dependency cycle error, the cycle must now be broken."
        );
    }

    for name in [
        "asyncAffectedTracking",
        "casOutputsCache",
        "explicitTaskOutputStyle",
        "nativeFileHashing",
    ] {
        remove_setting(root, &format!("experiments.{name}"));
    }

    remove_setting_if_empty(root, "experiments");

    rename_setting(
        root,
        "remote.cache.verifyIntegrity",
        "cache.cas.verifyIntegrity",
    );

    remove_setting_if_empty(root, "remote.cache");

    with_setting_parent(root, "vcs.provider", |node, name| {
        if node
            .get(name)
            .is_some_and(|value| value.as_str() == Some("bitbucket-legacy"))
        {
            node.insert(YamlValue::from(name), YamlValue::from("bitbucket"));
        }
    });
}

fn migrate_workspace_config_files(session: &MoonSession) -> miette::Result<()> {
    for config_path in session.config_loader.get_workspace_files() {
        if config_path.exists() {
            migrate_config_file(&config_path, migrate_workspace_setting)?;
        }
    }

    Ok(())
}

fn migrate_tasks_config_files(session: &MoonSession) -> miette::Result<()> {
    for config_path in session.config_loader.get_tasks_files(&session.config_dir)? {
        migrate_config_file(&config_path, migrate_tasks_setting)?;
    }

    Ok(())
}

fn migrate_project_config_files(session: &MoonSession) -> miette::Result<()> {
    for config_path in find_project_config_files(
        session,
        &["yml", "yaml", "jsonc", "json", "pkl", "hcl", "toml"],
    )? {
        migrate_config_file(&config_path, migrate_tasks_setting)?;
    }

    Ok(())
}
