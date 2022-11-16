use super::check_dirty_repo;
use crate::helpers::load_workspace;
use moon_config::{
    DependencyConfig, DependencyScope, PlatformType, ProjectConfig, ProjectDependsOn,
    TaskCommandArgs,
};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_error::MoonError;
use moon_logger::info;
use moon_node_lang::package::{DepsSet, PackageJson};
use moon_node_platform::create_tasks_from_scripts;
use moon_utils::yaml::{self, Hash, Yaml};
use rustc_hash::FxHashMap;
use serde_yaml::to_string;

// Don't use serde since it writes *everything*, which is a ton of nulled fields!
pub fn convert_to_yaml(config: &ProjectConfig) -> Result<Yaml, Box<dyn std::error::Error>> {
    let mut root = Hash::new();

    root.insert(
        Yaml::String("language".to_owned()),
        Yaml::String(to_string(&config.language)?.trim().to_owned()),
    );

    if !config.depends_on.is_empty() {
        let mut depends_on = vec![];

        for dep in &config.depends_on {
            match dep {
                ProjectDependsOn::String(value) => {
                    depends_on.push(Yaml::String(value.to_owned()));
                }
                ProjectDependsOn::Object(value) => {
                    let mut dep_value = Hash::new();

                    dep_value.insert(
                        Yaml::String("id".to_owned()),
                        Yaml::String(value.id.to_owned()),
                    );

                    dep_value.insert(
                        Yaml::String("scope".to_owned()),
                        Yaml::String(to_string(&value.scope)?),
                    );

                    if let Some(via) = &value.via {
                        dep_value
                            .insert(Yaml::String("via".to_owned()), Yaml::String(via.to_owned()));
                    }
                }
            }
        }

        root.insert(
            Yaml::String("dependsOn".to_owned()),
            Yaml::Array(depends_on),
        );
    }

    // We're only declaring fields used in `create_tasks_from_scripts`, not everything
    if !config.tasks.is_empty() {
        let mut tasks = Hash::new();

        let convert_string_list = |list: &Vec<String>| {
            Yaml::Array(list.iter().map(|v| Yaml::String(v.to_owned())).collect())
        };

        let convert_command_args = |value: &TaskCommandArgs| match value {
            TaskCommandArgs::String(v) => Yaml::String(v.to_owned()),
            TaskCommandArgs::Sequence(vs) => convert_string_list(vs),
        };

        for (id, task_config) in &config.tasks {
            let mut task = Hash::new();

            if let Some(command) = &task_config.command {
                task.insert(
                    Yaml::String("command".to_owned()),
                    convert_command_args(command),
                );
            }

            if let Some(args) = &task_config.args {
                task.insert(Yaml::String("args".to_owned()), convert_command_args(args));
            }

            if let Some(outputs) = &task_config.outputs {
                task.insert(
                    Yaml::String("outputs".to_owned()),
                    convert_string_list(outputs),
                );
            }
            if let Some(env) = &task_config.env {
                let mut env_vars = Hash::new();

                for (key, value) in env {
                    env_vars.insert(Yaml::String(key.to_owned()), Yaml::String(value.to_owned()));
                }

                task.insert(Yaml::String("env".to_owned()), Yaml::Hash(env_vars));
            }

            if !matches!(task_config.platform, PlatformType::Node) {
                task.insert(
                    Yaml::String("platform".to_owned()),
                    Yaml::String(to_string(&task_config.platform)?.trim().to_owned()),
                );
            }

            if task_config.local {
                task.insert(Yaml::String("local".to_owned()), Yaml::Boolean(true));
            }

            tasks.insert(Yaml::String(id.to_owned()), Yaml::Hash(task));
        }

        root.insert(Yaml::String("tasks".to_owned()), Yaml::Hash(tasks));
    }

    Ok(Yaml::Hash(root))
}

pub async fn from_package_json(
    project_id: &str,
    skip_touched_files_check: &bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    if *skip_touched_files_check {
        info!("Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };
    // Create a mapping of `package.json` names to project IDs
    let mut package_map: FxHashMap<String, String> = FxHashMap::default();

    for id in workspace.projects.ids() {
        let project = workspace.projects.load(&id)?;

        if let Some(package_json) = PackageJson::read(&project.root)? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, id);
            }
        }
    }

    // Create or update the local `moon.yml`
    let mut project = workspace.projects.load(project_id)?;

    let mut link_deps = |deps: &DepsSet, scope: DependencyScope| {
        for package_name in deps.keys() {
            if let Some(dep_id) = package_map.get(package_name) {
                project
                    .config
                    .depends_on
                    .push(if matches!(scope, DependencyScope::Production) {
                        ProjectDependsOn::String(dep_id.to_owned())
                    } else {
                        ProjectDependsOn::Object(DependencyConfig {
                            id: dep_id.to_owned(),
                            scope: scope.clone(),
                            via: None,
                        })
                    });
            }
        }
    };

    PackageJson::sync(&project.root, |package_json| {
        // Create tasks from `package.json` scripts
        for (task_id, task_config) in create_tasks_from_scripts(&project.id, package_json)
            .map_err(|e| MoonError::Generic(e.to_string()))?
        {
            project.config.tasks.insert(task_id, task_config);
        }

        // Link deps from `package.json` dependencies
        if let Some(deps) = &package_json.dependencies {
            link_deps(deps, DependencyScope::Production);
        }

        if let Some(deps) = &package_json.dev_dependencies {
            link_deps(deps, DependencyScope::Development);
        }

        if let Some(deps) = &package_json.peer_dependencies {
            link_deps(deps, DependencyScope::Peer);
        }

        Ok(())
    })?;

    yaml::write_with_config(
        project.root.join(CONFIG_PROJECT_FILENAME),
        convert_to_yaml(&project.config)?,
    )?;

    Ok(())
}
