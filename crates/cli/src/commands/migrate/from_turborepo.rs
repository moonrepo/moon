use super::check_dirty_repo;
use crate::helpers::AnyError;
use moon::{generate_project_graph, load_workspace};
use moon_config::{ProjectConfig, RunnerConfig, TaskCommandArgs, TaskConfig};
use moon_constants as constants;
use moon_logger::{info, warn};
use moon_terminal::safe_exit;
use moon_utils::{fs, json, yaml};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboTask {
    cache: Option<bool>,
    depends_on: Option<Vec<String>>,
    env: Option<Vec<String>>,
    inputs: Option<Vec<String>>,
    outputs: Option<Vec<String>>,
    persistent: Option<bool>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboJson {
    global_dependencies: Option<Vec<String>>,
    global_env: Option<Vec<String>>,
    pipeline: FxHashMap<String, TurboTask>,
}

pub fn extract_project_task_ids(key: &str) -> (Option<String>, String) {
    if key.contains('#') {
        let mut parts = key.split('#');

        return (
            Some(parts.next().unwrap().to_string()),
            parts.next().unwrap().to_string(),
        );
    }

    (None, key.to_owned())
}

pub fn convert_globals(turbo: &TurboJson, runner_config: &mut RunnerConfig) -> bool {
    let mut modified = false;

    if let Some(global_deps) = &turbo.global_dependencies {
        runner_config.implicit_inputs.extend(global_deps.to_owned());
        modified = true;
    }

    if let Some(global_env) = &turbo.global_env {
        for env in global_env {
            runner_config.implicit_inputs.push(format!("${}", env));
        }

        modified = true;
    }

    modified
}

pub fn convert_task(name: String, task: TurboTask) -> TaskConfig {
    let mut config = TaskConfig::default();
    let mut inputs = vec![];

    config.command = Some(TaskCommandArgs::String(format!(
        "moon node run-script {}",
        name
    )));

    if let Some(turbo_deps) = task.depends_on {
        let mut deps = vec![];

        for dep in turbo_deps {
            if dep.starts_with('^') {
                deps.push(dep.replace('^', "^:").to_owned());
            } else if dep.contains('#') {
                deps.push(dep.replace('#', ":").to_owned());
            } else if dep.starts_with('$') {
                inputs.push(dep);
            } else {
                deps.push(dep);
            }
        }

        if !deps.is_empty() {
            config.deps = Some(deps);
        }
    }

    if let Some(turbo_env) = task.env {
        for env in turbo_env {
            inputs.push(format!("${}", env));
        }
    }

    if let Some(turbo_inputs) = task.inputs {
        inputs.extend(turbo_inputs);
    }

    if let Some(turbo_outputs) = task.outputs {
        let mut outputs = vec![];

        for output in turbo_outputs {
            // We don't support globs at the moment
            if output.contains('*') {
                outputs.push(
                    output
                        .replace("/**/*", "")
                        .replace("/**", "")
                        .replace("/*", ""),
                );
            } else {
                outputs.push(output);
            }
        }

        if !outputs.is_empty() {
            config.outputs = Some(outputs);
        }
    }

    if !inputs.is_empty() {
        config.inputs = Some(inputs);
    }

    config.local = task.persistent.unwrap_or_default();
    config.options.cache = task.cache;

    config
}

pub async fn from_turborepo(skip_touched_files_check: &bool) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let turbo_file = workspace.root.join("turbo.json");

    if !turbo_file.exists() {
        eprintln!("No turbo.json was found in the current directory.");
        safe_exit(1);
    }

    if *skip_touched_files_check {
        info!("Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };

    let project_graph = generate_project_graph(&mut workspace).await?;
    let turbo_json: TurboJson = json::read(&turbo_file)?;

    // Convert globals first
    if convert_globals(&turbo_json, &mut workspace.config.runner) {
        yaml::write(
            workspace
                .root
                .join(constants::CONFIG_DIRNAME)
                .join(constants::CONFIG_WORKSPACE_FILENAME),
            &workspace.config,
        )?;
    }

    // Convert tasks second
    let mut has_warned_root_tasks = false;
    let mut has_modified_global_project = false;
    let mut modified_projects: FxHashMap<&PathBuf, ProjectConfig> = FxHashMap::default();

    for (id, task) in turbo_json.pipeline {
        if id.starts_with("//#") {
            if !has_warned_root_tasks {
                warn!("Unable to migrate root-level `//#` tasks. Create a root-level project manually to support similar functionality: https://moonrepo.dev/docs/guides/root-project");
                has_warned_root_tasks = true;
            }

            continue;
        }

        match extract_project_task_ids(&id) {
            (Some(project_id), task_id) => {
                let project = project_graph.get(&project_id)?;
                let task_config = convert_task(task_id.clone(), task);

                if let Some(config) = modified_projects.get_mut(&project.root) {
                    config.tasks.insert(task_id, task_config);
                } else {
                    let mut config = project.config.clone();
                    config.tasks.insert(task_id, task_config);

                    modified_projects.insert(&project.root, config);
                }
            }
            (None, task_id) => {
                workspace
                    .projects_config
                    .tasks
                    .insert(task_id.clone(), convert_task(task_id, task));
                has_modified_global_project = true;
            }
        }
    }

    if has_modified_global_project {
        yaml::write(
            workspace
                .root
                .join(constants::CONFIG_DIRNAME)
                .join(constants::CONFIG_GLOBAL_PROJECT_FILENAME),
            &workspace.projects_config,
        )?;
    }

    for (project_root, project_config) in modified_projects {
        yaml::write(
            project_root.join(constants::CONFIG_PROJECT_FILENAME),
            &project_config,
        )?;
    }

    fs::remove_file(&turbo_file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_utils::string_vec;

    mod globals_conversion {
        use super::*;

        #[test]
        fn converst_deps() {
            let mut config = RunnerConfig {
                implicit_inputs: string_vec!["existing.txt"],
                ..RunnerConfig::default()
            };

            convert_globals(
                &TurboJson {
                    global_dependencies: Some(string_vec!["file.ts", "glob/**/*.js"]),
                    ..TurboJson::default()
                },
                &mut config,
            );

            assert_eq!(
                config.implicit_inputs,
                string_vec!["existing.txt", "file.ts", "glob/**/*.js"]
            );
        }

        #[test]
        fn converst_env() {
            let mut config = RunnerConfig {
                implicit_inputs: string_vec!["$FOO"],
                ..RunnerConfig::default()
            };

            convert_globals(
                &TurboJson {
                    global_env: Some(string_vec!["BAR", "BAZ"]),
                    ..TurboJson::default()
                },
                &mut config,
            );

            assert_eq!(config.implicit_inputs, string_vec!["$FOO", "$BAR", "$BAZ"]);
        }
    }

    mod task_conversion {
        use super::*;

        #[test]
        fn sets_command() {
            let config = convert_task("foo".into(), TurboTask::default());

            assert_eq!(
                config.command.unwrap(),
                TaskCommandArgs::String("moon node run-script foo".into())
            );
        }

        #[test]
        fn converts_deps() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    depends_on: Some(string_vec!["normal", "^parent", "project#normal", "$VAR"]),
                    ..TurboTask::default()
                },
            );

            assert_eq!(
                config.deps.unwrap(),
                string_vec!["normal", "^:parent", "project:normal"]
            );
            assert_eq!(config.inputs.unwrap(), string_vec!["$VAR"]);
        }

        #[test]
        fn doesnt_set_deps_if_empty() {
            let config = convert_task("foo".into(), TurboTask::default());

            assert_eq!(config.deps, None);
        }

        #[test]
        fn converts_env_to_inputs() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    env: Some(string_vec!["FOO", "BAR"]),
                    ..TurboTask::default()
                },
            );

            assert_eq!(config.inputs.unwrap(), string_vec!["$FOO", "$BAR"]);
        }

        #[test]
        fn inherits_inputs() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    inputs: Some(string_vec!["file.ts", "some/folder", "some/glob/**/*"]),
                    ..TurboTask::default()
                },
            );

            assert_eq!(
                config.inputs.unwrap(),
                string_vec!["file.ts", "some/folder", "some/glob/**/*"]
            );
        }

        #[test]
        fn converts_outputs() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    outputs: Some(string_vec![
                        "dir",
                        "dir/**/*",
                        "dir/**",
                        "dir/*",
                        "dir/*/sub"
                    ]),
                    ..TurboTask::default()
                },
            );

            assert_eq!(
                config.outputs.unwrap(),
                string_vec!["dir", "dir", "dir", "dir", "dir/sub"]
            );
        }

        #[test]
        fn doesnt_set_outputs_if_empty() {
            let config = convert_task("foo".into(), TurboTask::default());

            assert_eq!(config.outputs, None);
        }

        #[test]
        fn sets_local() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    persistent: Some(true),
                    ..TurboTask::default()
                },
            );

            assert!(config.local);
        }

        #[test]
        fn sets_cache() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    cache: Some(false),
                    ..TurboTask::default()
                },
            );

            assert_eq!(config.options.cache, Some(false));
        }
    }
}
