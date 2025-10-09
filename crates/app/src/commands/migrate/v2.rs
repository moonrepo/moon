#![allow(clippy::collapsible_if, clippy::single_match)]

use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_common::consts::CONFIG_DIRNAME;
use moon_console::ui::Confirm;
use starbase::AppResult;
use starbase_utils::yaml::{self, YamlMapping, YamlValue};
use starbase_utils::{fs, glob};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct MigrateV2Args {
    #[arg(long, help = "Skip prompts and apply all migrations")]
    yes: bool,
}

#[instrument(skip_all)]
pub async fn v2(session: MoonSession, args: MigrateV2Args) -> AppResult {
    let skip_prompts = args.yes;

    // Configuration
    let mut confirmed = false;

    if !skip_prompts {
        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: "Migrate configuration files?".to_string(),
                    description: "This will strip comments and re-format the files.".to_string(),
                    on_confirm: &mut confirmed
                )
            })
            .await?;
    }

    if confirmed || skip_prompts {
        migrate_workspace_config(&session)?;
        migrate_toolchain_config_file(&session)?;
        migrate_tasks_config_files(&session)?;
    }

    Ok(None)
}

// CONFIGURATION

fn warn_pkl_config_files() {
    static PKL_WARNED: AtomicBool = AtomicBool::new(false);

    if !PKL_WARNED.load(Ordering::Relaxed) {
        warn!("Pkl based configuration files cannot be automatically migrated!");
        PKL_WARNED.store(true, Ordering::Release);
    }
}

fn load_config_file(config_path: &Path) -> miette::Result<YamlValue> {
    let content = fs::read_file(config_path)?
        .replace("$projectType", "$projectLayer")
        .replace("$taskPlatform", "$taskToolchain");

    let data: YamlValue = yaml::serde_yml::from_str(&content).into_diagnostic()?;

    Ok(data)
}

fn rename_setting(parent: &mut YamlMapping, old: &str, new: &str) {
    if let Some(value) = parent.get(old) {
        parent.insert(YamlValue::String(new.to_owned()), value.to_owned());
    }

    parent.remove_entry(old);
}

fn upsert_root_setting(root: &mut YamlMapping, root_key: &str, key: &str, value: &YamlValue) {
    root.entry(YamlValue::String(root_key.into()))
        .or_insert(YamlValue::Mapping(Default::default()))
        .as_mapping_mut()
        .expect("must be an object")
        .insert(YamlValue::String(key.to_owned()), value.to_owned());
}

fn migrate_task_setting(_key: &YamlValue, value: &mut YamlValue) {
    let fields = value.as_mapping_mut().expect("task must be an object");

    rename_setting(fields, "platform", "toolchain");
}

fn migrate_tasks_config_files(session: &MoonSession) -> miette::Result<()> {
    if session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("tasks.pkl")
        .exists()
    {
        warn_pkl_config_files();
    }

    for config_path in glob::walk_files(
        session.workspace_root.join(CONFIG_DIRNAME),
        ["tasks.yml", "tasks/**/*.yml"],
    )? {
        let mut config = load_config_file(&config_path)?;

        if let Some(root) = config.as_mapping_mut() {
            if let Some(tasks) = root
                .get_mut("tasks")
                .and_then(|tasks| tasks.as_mapping_mut())
            {
                for (id, task) in tasks {
                    migrate_task_setting(id, task);
                }
            }
        }

        yaml::write_file_with_config(&config_path, &config)?;
    }

    Ok(())
}

fn apply_to_javascript_setting(root: &mut YamlMapping, key: &str, value: &YamlValue) {
    // Remove
    if key.is_empty() || key == "packagesRoot" {
        return;
    }

    upsert_root_setting(
        root,
        "javascript",
        match key {
            "rootPackageOnly" => "rootPackageDependenciesOnly",
            _ => key,
        },
        value,
    );
}

fn migrate_toolchain_bun_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().expect("`bun` must be an object") {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Keep in toolchain
            "installArgs" | "plugin" | "version" => {
                upsert_root_setting(root, "bun", key, value);
            }
            // Move to javascript
            _ => {
                apply_to_javascript_setting(root, key, value);
            }
        }
    }
}

fn migrate_toolchain_deno_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().expect("`deno` must be an object") {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Remove
            "depsFile" | "lockfile" => {}
            // Keep in toolchain
            "installArgs" | "plugin" | "version" => {
                upsert_root_setting(root, "deno", key, value);
            }
            _ => {}
        }
    }
}

fn migrate_toolchain_node_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().expect("`node` must be an object") {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Remove
            "addEnginesConstraint" => {}
            // Rename
            "binExecArgs" => {
                upsert_root_setting(root, "node", "executeArgs", value);
            }
            // Keep in toolchain
            "installArgs" | "plugin" | "syncVersionManagerConfig" | "version" => {
                upsert_root_setting(root, "node", key, value);
            }
            // Move to own toolchain
            "bun" | "npm" | "pnpm" | "yarn" => {
                root.entry(YamlValue::String(key.into()))
                    .or_insert(YamlValue::Mapping(Default::default()))
                    .as_mapping_mut()
                    .expect("must be an object")
                    .extend(value.as_mapping().unwrap().to_owned());
            }
            // Move to javascript
            _ => {
                apply_to_javascript_setting(root, key, value);
            }
        }
    }
}

fn migrate_toolchain_config_file(session: &MoonSession) -> miette::Result<()> {
    if session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("toolchain.pkl")
        .exists()
    {
        warn_pkl_config_files();
    }

    let config_path = session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("toolchain.yml");

    if !config_path.exists() {
        return Ok(());
    }

    let old_data: YamlValue = load_config_file(&config_path)?;
    let mut new_data = YamlMapping::default();

    if let Some(root) = old_data.as_mapping() {
        for (base_key, setting) in root {
            let Some(key) = base_key.as_str() else {
                continue;
            };

            match key {
                "bun" => {
                    migrate_toolchain_bun_setting(&mut new_data, setting);
                }

                "deno" => {
                    migrate_toolchain_deno_setting(&mut new_data, setting);
                }

                "node" => {
                    migrate_toolchain_node_setting(&mut new_data, setting);
                }

                "python" => {
                    // TODO
                }

                _ => {
                    // Move unstable to stable
                    if let Some(suffix) = key.strip_prefix("unstable_") {
                        new_data.insert(YamlValue::String(suffix.into()), setting.to_owned());
                    }
                    // Other setting, keep as-is
                    else {
                        new_data.insert(base_key.to_owned(), setting.to_owned());
                    }
                }
            };
        }
    }

    yaml::write_file_with_config(&config_path, &YamlValue::Mapping(new_data))?;

    Ok(())
}

fn migrate_workspace_config(session: &MoonSession) -> miette::Result<()> {
    if session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("workspace.pkl")
        .exists()
    {
        warn_pkl_config_files();
    }

    let config_path = session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("workspace.yml");

    if !config_path.exists() {
        return Ok(());
    }

    let mut config: YamlValue = load_config_file(&config_path)?;

    if let Some(root) = config.as_mapping_mut() {
        for (key, value) in root {
            match key.as_str().unwrap_or_default() {
                "constraints" => {
                    rename_setting(
                        value
                            .as_mapping_mut()
                            .expect("`constraints` must be an object"),
                        "enforceProjectTypeRelationships",
                        "enforceLayerRelationships",
                    );
                }
                _ => {}
            };
        }
    }

    yaml::write_file_with_config(&config_path, &config)?;

    Ok(())
}
