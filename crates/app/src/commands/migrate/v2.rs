use super::utils::*;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_config::{LayerType, StackType};
use starbase_utils::yaml::{YamlMapping, YamlValue};
use starbase_utils::{fs, glob};
use std::str::FromStr;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct MigrateV2Args {
    #[arg(long, help = "Skip prompts and apply all migrations")]
    yes: bool,
}

#[instrument(skip(session))]
pub async fn v2(session: MoonSession, args: MigrateV2Args) -> SessionResult {
    if confirm_config_migration(&session, args.yes).await? {
        migrate_workspace_config_file(&session)?;
        migrate_toolchain_config_file(&session)?;
        migrate_tasks_config_files(&session)?;
        migrate_project_config_files(&session)?;
    }

    Ok(None)
}

// CONFIGURATION

fn migrate_tokens(value: &mut YamlValue) {
    match value {
        YamlValue::String(string) => {
            *string = string
                .replace("$projectName", "$projectTitle")
                .replace("$projectType", "$projectLayer")
                .replace("$taskPlatform", "$taskToolchain");
        }
        YamlValue::Sequence(items) => {
            for item in items {
                migrate_tokens(item);
            }
        }
        YamlValue::Mapping(map) => {
            for (_, item) in map.iter_mut() {
                migrate_tokens(item);
            }
        }
        YamlValue::Tagged(tagged) => {
            migrate_tokens(&mut tagged.value);
        }
        _ => {}
    };
}

fn migrate_root_tokens(root: &mut YamlMapping) {
    for (_, value) in root.iter_mut() {
        migrate_tokens(value);
    }
}

fn upsert_root_setting(root: &mut YamlMapping, root_key: &str, key: &str, value: &YamlValue) {
    set_setting(root, &format!("{root_key}.{key}"), value.to_owned());
}

fn migrate_task_setting(task: &mut YamlValue) {
    let Some(task) = task.as_mapping_mut() else {
        return;
    };

    // Not exhaustive!
    let is_script = task
        .get("command")
        .and_then(YamlValue::as_str)
        .is_some_and(|command| {
            ["||", "&&", " | ", "; ", " > ", " >> ", " < "]
                .iter()
                .any(|pat| command.contains(pat))
        });

    if is_script {
        rename_setting(task, "command", "script");
    }

    rename_setting(task, "platform", "toolchains");

    let is_local = task.get("local").and_then(YamlValue::as_bool) == Some(true);

    if is_local && !task.contains_key("preset") {
        rename_key(task, "local", "preset");
        set_setting(task, "preset", YamlValue::from("server"));
    } else {
        remove_setting(task, "local");
    }

    if task
        .get("preset")
        .is_some_and(|value| value.as_str() == Some("watcher"))
    {
        remove_setting(task, "preset");
    }

    if let Some(YamlValue::Sequence(deps)) = task.get_mut("deps") {
        for dep in deps.iter_mut().filter_map(YamlValue::as_mapping_mut) {
            if let Some(YamlValue::String(args)) = dep.get("args") {
                let args = args.split_whitespace().map(YamlValue::from).collect();

                set_setting(dep, "args", YamlValue::Sequence(args));
            }
        }
    }
}

fn migrate_inherited_by_setting(file_name: &str) -> Option<YamlValue> {
    if file_name.is_empty() {
        return None;
    }

    let mut map = YamlMapping::new();

    if let Some(tag) = file_name.strip_prefix("tag-") {
        map.insert(
            YamlValue::String("tag".into()),
            YamlValue::String(tag.into()),
        );
    } else {
        for part in file_name.split('-') {
            if part == "bun"
                || part == "deno"
                || part == "node"
                || part == "javascript"
                || part == "typescript"
                || part == "rust"
                || part == "python"
                || part == "go"
            {
                map.insert(
                    YamlValue::String("toolchain".into()),
                    YamlValue::String(part.into()),
                );
            } else if StackType::from_str(part).is_ok() {
                map.insert(
                    YamlValue::String("stack".into()),
                    YamlValue::String(part.into()),
                );
            } else if LayerType::from_str(part).is_ok() {
                map.insert(
                    YamlValue::String("layer".into()),
                    YamlValue::String(part.into()),
                );
            } else {
                map.insert(
                    YamlValue::String("language".into()),
                    YamlValue::String(part.into()),
                );
            }
        }
    }

    Some(YamlValue::Mapping(map))
}

fn migrate_tasks_config_files(session: &MoonSession) -> miette::Result<()> {
    for config_path in glob::walk_files(
        &session.config_dir,
        ["tasks.{pkl,yml}", "tasks/**/*.{pkl,yml}"],
    )? {
        // The root file is inherited by all projects
        let is_root = config_path.parent() == Some(session.config_dir.as_path());
        let file_name = config_path
            .file_stem()
            .and_then(|file| file.to_str())
            .unwrap_or_default();

        migrate_config_file(&config_path, |root, _| {
            migrate_root_tokens(root);

            if !is_root
                && !root.contains_key("inheritedBy")
                && let Some(by) = migrate_inherited_by_setting(file_name)
            {
                set_setting(root, "inheritedBy", by);
            }

            if let Some(tasks) = root.get_mut("tasks").and_then(YamlValue::as_mapping_mut) {
                for (_, task) in tasks.iter_mut() {
                    migrate_task_setting(task);
                }
            }
        })?;
    }

    // Rename the old file to a new one
    for config_path in glob::walk_files(&session.config_dir, ["tasks.{pkl,yml}"])? {
        let ext = config_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("yml");

        fs::rename(
            &config_path,
            session.config_dir.join("tasks").join(format!("all.{ext}")),
        )?;
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
                    .or_insert_with(|| YamlValue::Mapping(Default::default()))
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

fn migrate_toolchain_python_setting(root: &mut YamlMapping, setting: &YamlValue) {
    for (base_key, value) in setting.as_mapping().expect("`python` must be an object") {
        let key = base_key.as_str().unwrap_or_default();

        match key {
            // Remove
            "rootRequirementsOnly" | "rootVenvOnly" => {}
            // Keep in toolchain
            "plugin" | "version" | "venvName" => {
                upsert_root_setting(root, "unstable_python", key, value);
            }
            // Move to own toolchain
            "pip" | "uv" => {
                root.entry(YamlValue::String(format!("unstable_{key}")))
                    .or_insert_with(|| YamlValue::Mapping(Default::default()))
                    .as_mapping_mut()
                    .expect("must be an object")
                    .extend(value.as_mapping().unwrap().to_owned());
            }
            _ => {}
        }
    }
}

fn migrate_toolchain_config_file(session: &MoonSession) -> miette::Result<()> {
    if session.config_dir.join("toolchain.pkl").exists() {
        warn_pkl_config_files();
    }

    let config_path = session.config_dir.join("toolchain.yml");

    if !config_path.exists() {
        return Ok(());
    }

    let Some(old_data) = read_config_file(&config_path) else {
        return Ok(());
    };

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
                    migrate_toolchain_python_setting(&mut new_data, setting);
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

    migrate_root_tokens(&mut new_data);

    // Write the new file before removing the old one
    write_config_file(
        &session.config_dir.join("toolchains.yml"),
        &YamlValue::Mapping(new_data),
    )?;

    fs::remove_file(config_path)?;

    Ok(())
}

fn migrate_workspace_config_file(session: &MoonSession) -> miette::Result<()> {
    let mut extensions = None;

    for config_path in [
        session.config_dir.join("workspace.pkl"),
        session.config_dir.join("workspace.yml"),
    ] {
        if !config_path.exists() {
            continue;
        }

        migrate_config_file(&config_path, |root, _| {
            migrate_root_tokens(root);

            with_setting_parent(root, "codeowners.orderBy", |node, name| {
                if node
                    .get(name)
                    .is_some_and(|value| value.as_str() == Some("project-name"))
                {
                    node.insert(YamlValue::from(name), YamlValue::from("project-id"));
                }
            });
            rename_setting(root, "codeowners.syncOnRun", "codeowners.sync");
            rename_setting(
                root,
                "constraints.enforceProjectTypeRelationships",
                "constraints.enforceLayerRelationships",
            );
            rename_setting(
                root,
                "docker.prune.installToolchainDeps",
                "docker.prune.installToolchainDependencies",
            );
            remove_setting(root, "docker.scaffold.copyToolchainFiles");
            rename_setting(
                root,
                "docker.scaffold.include",
                "docker.scaffold.configsPhaseGlobs",
            );
            remove_setting(root, "experiments");
            extensions = remove_setting(root, "extensions");
            remove_setting(root, "hasher.batchSize");
            rename_setting(root, "runner", "pipeline");
            remove_setting(root, "pipeline.archivableTargets");
            rename_setting(root, "unstable_remote", "remote");
            rename_setting(root, "vcs.manager", "vcs.client");
            rename_setting(root, "vcs.syncHooks", "vcs.sync");
        })?;
    }

    if let Some(extensions) = extensions {
        write_config_file(&session.config_dir.join("extensions.yml"), &extensions)?;
    }

    Ok(())
}

fn migrate_project_config_files(session: &MoonSession) -> miette::Result<()> {
    for config_path in find_project_config_files(session, &["pkl", "yml"])? {
        migrate_config_file(&config_path, |root, _| {
            migrate_root_tokens(root);

            rename_setting(
                root,
                "docker.scaffold.include",
                "docker.scaffold.sourcesPhaseGlobs",
            );
            rename_setting(root, "toolchain", "toolchains");
            rename_setting(root, "platform", "toolchains.default");
            rename_setting(root, "type", "layer");

            if let Some(project) = root.get_mut("project").and_then(YamlValue::as_mapping_mut) {
                rename_setting(project, "name", "title");

                if let Some(YamlValue::Mapping(metadata)) = remove_setting(project, "metadata") {
                    project.extend(metadata);
                }
            }

            if let Some(toolchains) = root
                .get_mut("toolchains")
                .and_then(YamlValue::as_mapping_mut)
            {
                for (_, toolchain) in toolchains.iter_mut() {
                    if let Some(map) = toolchain.as_mapping_mut() {
                        remove_setting(map, "disabled");
                    }
                }
            }

            if let Some(tasks) = root.get_mut("tasks").and_then(YamlValue::as_mapping_mut) {
                for (_, task) in tasks.iter_mut() {
                    migrate_task_setting(task);
                }
            }
        })?;
    }

    Ok(())
}
