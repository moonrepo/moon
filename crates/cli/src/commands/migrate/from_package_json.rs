use super::check_dirty_repo;
use crate::helpers::AnyError;
use moon::{generate_project_graph, load_workspace};
use moon_config::{
    DependencyConfig, DependencyScope, PlatformType, ProjectConfig, ProjectDependsOn,
    TaskCommandArgs,
};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_error::MoonError;
use moon_logger::info;
use moon_node_lang::package::{DepsSet, PackageJson};
use moon_node_platform::create_tasks_from_scripts;
use moon_utils::yaml::{self, Mapping, YamlValue};
use rustc_hash::FxHashMap;
use serde_yaml::to_string;

// Don't use serde since it writes *everything*, which is a ton of nulled fields!
pub fn convert_to_yaml(config: &ProjectConfig) -> Result<YamlValue, AnyError> {
    let mut root = Mapping::new();

    root.insert(
        YamlValue::String("language".to_owned()),
        YamlValue::String(to_string(&config.language)?.trim().to_owned()),
    );

    if !config.depends_on.is_empty() {
        let mut depends_on = vec![];

        for dep in &config.depends_on {
            match dep {
                ProjectDependsOn::String(value) => {
                    depends_on.push(YamlValue::String(value.to_owned()));
                }
                ProjectDependsOn::Object(value) => {
                    let mut dep_value = Mapping::new();

                    dep_value.insert(
                        YamlValue::String("id".to_owned()),
                        YamlValue::String(value.id.to_owned()),
                    );

                    dep_value.insert(
                        YamlValue::String("scope".to_owned()),
                        YamlValue::String(to_string(&value.scope)?),
                    );

                    if let Some(via) = &value.via {
                        dep_value.insert(
                            YamlValue::String("via".to_owned()),
                            YamlValue::String(via.to_owned()),
                        );
                    }
                }
            }
        }

        root.insert(
            YamlValue::String("dependsOn".to_owned()),
            YamlValue::Sequence(depends_on),
        );
    }

    // We're only declaring fields used in `create_tasks_from_scripts`, not everything
    if !config.tasks.is_empty() {
        let mut tasks = Mapping::new();

        let convert_string_list = |list: &Vec<String>| {
            YamlValue::Sequence(
                list.iter()
                    .map(|v| YamlValue::String(v.to_owned()))
                    .collect(),
            )
        };

        let convert_command_args = |value: &TaskCommandArgs| match value {
            TaskCommandArgs::String(v) => YamlValue::String(v.to_owned()),
            TaskCommandArgs::Sequence(vs) => convert_string_list(vs),
        };

        for (id, task_config) in &config.tasks {
            let mut task = Mapping::new();

            if let Some(command) = &task_config.command {
                task.insert(
                    YamlValue::String("command".to_owned()),
                    convert_command_args(command),
                );
            }

            if let Some(args) = &task_config.args {
                task.insert(
                    YamlValue::String("args".to_owned()),
                    convert_command_args(args),
                );
            }

            if let Some(outputs) = &task_config.outputs {
                task.insert(
                    YamlValue::String("outputs".to_owned()),
                    convert_string_list(outputs),
                );
            }
            if let Some(env) = &task_config.env {
                let mut env_vars = Mapping::new();

                for (key, value) in env {
                    env_vars.insert(
                        YamlValue::String(key.to_owned()),
                        YamlValue::String(value.to_owned()),
                    );
                }

                task.insert(
                    YamlValue::String("env".to_owned()),
                    YamlValue::Mapping(env_vars),
                );
            }

            if !matches!(task_config.platform, PlatformType::Node) {
                task.insert(
                    YamlValue::String("platform".to_owned()),
                    YamlValue::String(to_string(&task_config.platform)?.trim().to_owned()),
                );
            }

            if task_config.local {
                task.insert(YamlValue::String("local".to_owned()), YamlValue::Bool(true));
            }

            tasks.insert(YamlValue::String(id.to_owned()), YamlValue::Mapping(task));
        }

        root.insert(
            YamlValue::String("tasks".to_owned()),
            YamlValue::Mapping(tasks),
        );
    }

    Ok(YamlValue::Mapping(root))
}

pub async fn from_package_json(
    project_id: &str,
    skip_touched_files_check: &bool,
) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;

    if *skip_touched_files_check {
        info!("Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };

    // Create a mapping of `package.json` names to project IDs
    let project_graph = generate_project_graph(&mut workspace)?;
    let mut package_map: FxHashMap<String, String> = FxHashMap::default();

    for project in project_graph.get_all()? {
        if let Some(package_json) = PackageJson::read(&project.root)? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, project.id.to_owned());
            }
        }
    }

    // Create or update the local `moon.yml`
    let mut project = project_graph.get(project_id)?.to_owned();

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
