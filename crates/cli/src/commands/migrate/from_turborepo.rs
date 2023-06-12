use super::check_dirty_repo;
use moon::{generate_project_graph, load_workspace};
use moon_common::{consts, Id};
use moon_config::{
    InputPath, OutputPath, PartialInheritedTasksConfig, PartialProjectConfig, PartialTaskConfig,
    PartialTaskOptionsConfig, PlatformType, ProjectConfig, TaskCommandArgs,
};
use moon_logger::{info, warn};
use moon_target::Target;
use moon_terminal::safe_exit;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase::AppResult;
use starbase_utils::{fs, json, yaml};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

const LOG_TARGET: &str = "moon:migrate:from-turborepo";

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboTask {
    pub cache: Option<bool>,
    pub depends_on: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub inputs: Option<Vec<String>>,
    pub outputs: Option<Vec<String>>,
    pub persistent: Option<bool>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboJson {
    pub global_dependencies: Option<Vec<String>>,
    pub global_env: Option<Vec<String>>,
    pub pipeline: FxHashMap<String, TurboTask>,
}

pub fn extract_project_task_ids(key: &str) -> (Option<Id>, Id) {
    if key.contains('#') {
        let mut parts = key.split('#');

        return (
            Some(Id::raw(parts.next().unwrap())),
            Id::raw(parts.next().unwrap()),
        );
    }

    (None, Id::raw(key))
}

pub fn convert_globals(
    turbo: &TurboJson,
    tasks_config: &mut PartialInheritedTasksConfig,
) -> AppResult<bool> {
    let mut modified = false;

    if let Some(global_deps) = &turbo.global_dependencies {
        let implicit_inputs = tasks_config.implicit_inputs.get_or_insert(vec![]);

        for dep in global_deps {
            implicit_inputs.push(InputPath::from_str(dep)?);
        }

        modified = true;
    }

    if let Some(global_env) = &turbo.global_env {
        for env in global_env {
            tasks_config
                .implicit_inputs
                .get_or_insert(vec![])
                .push(InputPath::EnvVar(env.to_owned()));
        }

        modified = true;
    }

    Ok(modified)
}

pub fn convert_task(name: Id, task: TurboTask) -> AppResult<PartialTaskConfig> {
    let mut config = PartialTaskConfig::default();
    let mut inputs = vec![];

    config.command = Some(TaskCommandArgs::String(format!(
        "moon node run-script {name}"
    )));

    if let Some(turbo_deps) = task.depends_on {
        let mut deps: Vec<Target> = vec![];

        for dep in turbo_deps {
            if dep.starts_with('^') {
                deps.push(Target::parse(&dep.replace('^', "^:"))?);
            } else if dep.contains('#') {
                deps.push(Target::parse(&dep.replace('#', ":"))?);
            } else if dep.starts_with('$') {
                inputs.push(InputPath::from_str(&dep)?);
            } else {
                deps.push(Target::parse(&dep)?);
            }
        }

        if !deps.is_empty() {
            config.deps = Some(deps);
        }
    }

    if let Some(turbo_env) = task.env {
        for env in turbo_env {
            inputs.push(InputPath::EnvVar(env));
        }
    }

    if let Some(turbo_inputs) = task.inputs {
        for input in turbo_inputs {
            inputs.push(InputPath::from_str(&input)?);
        }
    }

    if let Some(turbo_outputs) = task.outputs {
        let mut outputs = vec![];

        for output in turbo_outputs {
            if output.ends_with("/**") {
                outputs.push(OutputPath::ProjectGlob(format!("{output}/*")));
            } else {
                outputs.push(OutputPath::from_str(&output)?);
            }
        }

        if !outputs.is_empty() {
            config.outputs = Some(outputs);
        }
    }

    if !inputs.is_empty() {
        config.inputs = Some(inputs);
    }

    config.platform = Some(PlatformType::Node);

    if task.persistent == Some(true) {
        config.local = task.persistent;
    }

    if task.cache == Some(false) {
        config
            .options
            .get_or_insert(PartialTaskOptionsConfig::default())
            .cache = task.cache;
    }

    Ok(config)
}

pub async fn from_turborepo(skip_touched_files_check: bool) -> AppResult {
    let mut workspace = load_workspace().await?;
    let turbo_file = workspace.root.join("turbo.json");

    if !turbo_file.exists() {
        eprintln!("No turbo.json was found in the workspace root.");
        safe_exit(1);
    }

    if skip_touched_files_check {
        info!(target: LOG_TARGET, "Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };

    let project_graph = generate_project_graph(&mut workspace).await?;
    let turbo_json: TurboJson = json::read_file(&turbo_file)?;
    let mut node_tasks_config = PartialInheritedTasksConfig::default();
    let mut has_modified_global_tasks = false;

    // Convert globals first
    if convert_globals(&turbo_json, &mut node_tasks_config)? {
        has_modified_global_tasks = true;
    }

    // Convert tasks second
    let mut has_warned_root_tasks = false;
    let mut modified_projects: FxHashMap<&PathBuf, PartialProjectConfig> = FxHashMap::default();

    for (id, task) in turbo_json.pipeline {
        if id.starts_with("//#") {
            if !has_warned_root_tasks {
                warn!(
                    target: LOG_TARGET,
                    "Unable to migrate root-level `//#` tasks. Create a root-level project manually to support similar functionality: https://moonrepo.dev/docs/guides/root-project"
                );
                has_warned_root_tasks = true;
            }

            continue;
        }

        match extract_project_task_ids(&id) {
            (Some(project_id), task_id) => {
                let project = project_graph.get(&project_id)?;
                let task_config = convert_task(task_id.clone(), task)?;

                if let Some(project_config) = modified_projects.get_mut(&project.root) {
                    project_config
                        .tasks
                        .get_or_insert(BTreeMap::new())
                        .insert(task_id, task_config);
                } else {
                    let mut project_config = ProjectConfig::load_partial(&project.root)?;

                    project_config
                        .tasks
                        .get_or_insert(BTreeMap::new())
                        .insert(task_id, task_config);

                    modified_projects.insert(&project.root, project_config);
                }
            }
            (None, task_id) => {
                node_tasks_config
                    .tasks
                    .get_or_insert(BTreeMap::new())
                    .insert(task_id.clone(), convert_task(task_id, task)?);
                has_modified_global_tasks = true;
            }
        }
    }

    if has_modified_global_tasks {
        let tasks_dir = workspace.root.join(consts::CONFIG_DIRNAME).join("tasks");

        if !tasks_dir.exists() {
            fs::create_dir_all(&tasks_dir)?;
        }

        yaml::write_with_config(tasks_dir.join("node.yml"), &node_tasks_config)?;
    }

    for (project_root, project_config) in modified_projects {
        yaml::write_with_config(
            project_root.join(consts::CONFIG_PROJECT_FILENAME),
            &project_config,
        )?;
    }

    fs::remove_file(&turbo_file)?;

    println!("Successfully migrated from Turborepo to moon!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_utils::string_vec;

    mod globals_conversion {
        use super::*;

        #[test]
        fn converts_deps() {
            let mut config = PartialInheritedTasksConfig {
                implicit_inputs: Some(vec![InputPath::ProjectFile("existing.txt".into())]),
                ..PartialInheritedTasksConfig::default()
            };

            convert_globals(
                &TurboJson {
                    global_dependencies: Some(string_vec!["file.ts", "glob/**/*.js"]),
                    ..TurboJson::default()
                },
                &mut config,
            )
            .unwrap();

            assert_eq!(
                config.implicit_inputs,
                Some(vec![
                    InputPath::ProjectFile("existing.txt".into()),
                    InputPath::ProjectFile("file.ts".into()),
                    InputPath::ProjectGlob("glob/**/*.js".into())
                ])
            );
        }

        #[test]
        fn converst_env() {
            let mut config = PartialInheritedTasksConfig {
                implicit_inputs: Some(vec![InputPath::EnvVar("FOO".into())]),
                ..PartialInheritedTasksConfig::default()
            };

            convert_globals(
                &TurboJson {
                    global_env: Some(string_vec!["BAR", "BAZ"]),
                    ..TurboJson::default()
                },
                &mut config,
            )
            .unwrap();

            assert_eq!(
                config.implicit_inputs,
                Some(vec![
                    InputPath::EnvVar("FOO".into()),
                    InputPath::EnvVar("BAR".into()),
                    InputPath::EnvVar("BAZ".into())
                ])
            );
        }
    }

    mod task_conversion {
        use super::*;

        #[test]
        fn sets_command() {
            let config = convert_task("foo".into(), TurboTask::default()).unwrap();

            assert_eq!(
                config.command,
                Some(TaskCommandArgs::String("moon node run-script foo".into()))
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
            )
            .unwrap();

            assert_eq!(
                config.deps,
                Some(vec![
                    Target::new_self("normal").unwrap(),
                    Target::parse("^:parent").unwrap(),
                    Target::parse("project:normal").unwrap(),
                ])
            );
            assert_eq!(
                config.inputs.unwrap(),
                vec![InputPath::EnvVar("VAR".into())]
            );
        }

        #[test]
        fn doesnt_set_deps_if_empty() {
            let config = convert_task("foo".into(), TurboTask::default()).unwrap();

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
            )
            .unwrap();

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    InputPath::EnvVar("FOO".into()),
                    InputPath::EnvVar("BAR".into())
                ]
            );
        }

        #[test]
        fn inherits_inputs() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    inputs: Some(string_vec!["file.ts", "some/folder", "some/glob/**/*"]),
                    ..TurboTask::default()
                },
            )
            .unwrap();

            assert_eq!(
                config.inputs.unwrap(),
                vec![
                    InputPath::ProjectFile("file.ts".into()),
                    InputPath::ProjectFile("some/folder".into()),
                    InputPath::ProjectGlob("some/glob/**/*".into()),
                ]
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
            )
            .unwrap();

            assert_eq!(
                config.outputs.unwrap(),
                vec![
                    OutputPath::ProjectFile("dir".into()),
                    OutputPath::ProjectGlob("dir/**/*".into()),
                    OutputPath::ProjectGlob("dir/**/*".into()),
                    OutputPath::ProjectGlob("dir/*".into()),
                    OutputPath::ProjectGlob("dir/*/sub".into()),
                ]
            );
        }

        #[test]
        fn doesnt_set_outputs_if_empty() {
            let config = convert_task("foo".into(), TurboTask::default()).unwrap();

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
            )
            .unwrap();

            assert_eq!(config.local, Some(true));
        }

        #[test]
        fn sets_cache_if_false() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    cache: Some(false),
                    ..TurboTask::default()
                },
            )
            .unwrap();

            assert_eq!(config.options.unwrap().cache, Some(false));
        }

        #[test]
        fn doesnt_set_cache_if_true() {
            let config = convert_task(
                "foo".into(),
                TurboTask {
                    cache: Some(true),
                    ..TurboTask::default()
                },
            )
            .unwrap();

            assert_eq!(config.options, None);
        }
    }
}
