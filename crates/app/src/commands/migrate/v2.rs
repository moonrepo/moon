use crate::session::MoonSession;
use clap::Args;
use miette::IntoDiagnostic;
use moon_common::consts::CONFIG_DIRNAME;
use starbase::AppResult;
use starbase_utils::fs;
use starbase_utils::yaml::{self, YamlMapping, YamlValue};
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct MigrateV2Args {}

#[instrument(skip_all)]
pub async fn v2(session: MoonSession, args: MigrateV2Args) -> AppResult {
    // Configuration
    migrate_toolchain_config(&session)?;

    Ok(None)
}

// CONFIGURATION

fn replace_config_tokens(content: String) -> String {
    content
        .replace("$projectType", "$projectLayer")
        .replace("$taskPlatform", "$taskToolchain")
}

fn add_to_root_setting(root: &mut YamlMapping, root_key: &str, key: &str, value: &YamlValue) {
    root.entry(YamlValue::String(root_key.into()))
        .or_insert(YamlValue::Mapping(Default::default()))
        .as_mapping_mut()
        .unwrap()
        .insert(YamlValue::String(key.to_owned()), value.to_owned());
}

fn apply_to_javascript_setting(root: &mut YamlMapping, key: &str, value: &YamlValue) {
    // Remove
    if key.is_empty() || key == "packagesRoot" {
        return;
    }

    add_to_root_setting(
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
    for (base_key, value) in setting.as_mapping().unwrap() {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Keep in toolchain
            "installArgs" | "plugin" | "version" => {
                add_to_root_setting(root, "bun", key, value);
            }
            // Move to javascript
            _ => {
                apply_to_javascript_setting(root, key, value);
            }
        }
    }
}

fn migrate_toolchain_deno_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().unwrap() {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Remove
            "depsFile" | "lockfile" => {}
            // Keep in toolchain
            "installArgs" | "plugin" | "version" => {
                add_to_root_setting(root, "deno", key, value);
            }
            _ => {}
        }
    }
}

fn migrate_toolchain_node_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().unwrap() {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Remove
            "addEnginesConstraint" => {}
            // Rename
            "binExecArgs" => {
                add_to_root_setting(root, "node", "executeArgs", value);
            }
            // Keep in toolchain
            "installArgs" | "plugin" | "syncVersionManagerConfig" | "version" => {
                add_to_root_setting(root, "node", key, value);
            }
            // Move to own toolchain
            "bun" | "npm" | "pnpm" | "yarn" => {
                root.entry(YamlValue::String(key.into()))
                    .or_insert(YamlValue::Mapping(Default::default()))
                    .as_mapping_mut()
                    .unwrap()
                    .extend(value.as_mapping().unwrap().to_owned());
            }
            // Move to javascript
            _ => {
                apply_to_javascript_setting(root, key, value);
            }
        }
    }
}

fn migrate_toolchain_config(session: &MoonSession) -> miette::Result<()> {
    let config_path = session
        .workspace_root
        .join(CONFIG_DIRNAME)
        .join("toolchain.yml");

    if !config_path.exists() {
        return Ok(());
    }

    // Replace static values first
    let mut content = fs::read_file(&config_path)?;

    content = replace_config_tokens(content);

    // Replace dynamic values second
    let old_data: YamlValue = yaml::serde_yml::from_str(&content).into_diagnostic()?;
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
