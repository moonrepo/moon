use crate::tasks_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::path::{to_virtual_string, WorkspaceRelativePathBuf};
use moon_common::{color, Id};
use moon_config::{patterns, InputPath};
use moon_project::Project;
use moon_task::{Target, TargetScope, Task};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::env;
use std::mem;
use std::path::Path;
use std::sync::Arc;
use tracing::{trace, warn};

fn substitute_env_var(value: &str, env_map: &FxHashMap<String, String>) -> String {
    patterns::ENV_VAR_SUBSTITUTE.replace_all(
        value,
        |caps: &patterns::Captures| {
            let name = caps.get(1).unwrap().as_str();
            let value = match env_map.get(name) {
                Some(var) => var.to_owned(),
                None => env::var(name).unwrap_or_default(),
            };

            if value.is_empty() {
                warn!(
                    "Task value `{}` contains the environment variable ${}, but this variable is either not set or is empty. Substituting with an empty string.",
                    value,
                    name
                );
            }

            value
        })
    .to_string()
}

pub struct TasksExpander<'proj> {
    pub project: &'proj mut Project,
    pub workspace_root: &'proj Path,
}

impl<'proj> TasksExpander<'proj> {
    pub fn expand<F>(project: &mut Project, workspace_root: &Path, query: F) -> miette::Result<()>
    where
        F: Fn(String) -> miette::Result<Vec<Arc<Project>>>,
    {
        let mut tasks = BTreeMap::new();
        let old_tasks = mem::take(&mut project.tasks);

        let mut expander = TasksExpander {
            project,
            workspace_root,
        };

        // Use `mem::take` so that we can mutably borrow the project and tasks in parallel
        for (task_id, mut task) in old_tasks {
            // Resolve in this order!
            expander.expand_env(&mut task)?;
            expander.expand_deps(&mut task, &query)?;
            expander.expand_inputs(&mut task)?;
            expander.expand_outputs(&mut task)?;
            expander.expand_args(&mut task)?;
            expander.expand_command(&mut task)?;

            tasks.insert(task_id, task);
        }

        project.tasks.extend(tasks);

        Ok(())
    }

    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            target = task.target.as_str(),
            command = &task.command,
            "Expanding tokens and variables in command"
        );

        // Token variables
        let command =
            TokenExpander::for_command(self.project, task, self.workspace_root).expand_command()?;

        // Environment variables
        task.command = substitute_env_var(&command, &task.env);

        Ok(())
    }

    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.args.is_empty() {
            return Ok(());
        }

        let handle_path = |path: WorkspaceRelativePathBuf| -> miette::Result<String> {
            if task.options.run_from_workspace_root {
                Ok(format!("./{}", path))
            } else if let Ok(proj_path) = path.strip_prefix(&self.project.source) {
                Ok(format!("./{}", proj_path))
            } else {
                to_virtual_string(path.to_logical_path(self.workspace_root))
            }
        };

        trace!(
            target = task.target.as_str(),
            args = ?task.args,
            "Expanding tokens and variables in args",
        );

        // Expand inline tokens
        let mut args = vec![];
        let expander = TokenExpander::for_args(self.project, task, self.workspace_root);

        for arg in &task.args {
            // Token functions
            if expander.has_token_function(arg) {
                let (files, globs) = expander.replace_function(arg)?;

                for file in files {
                    args.push(handle_path(file)?);
                }

                for glob in globs {
                    args.push(handle_path(glob)?);
                }

            // Token variables
            } else if expander.has_token_variable(arg) {
                args.push(expander.replace_variables(arg)?);

            // Environment variables
            } else if patterns::ENV_VAR_SUBSTITUTE.is_match(arg) {
                args.push(substitute_env_var(arg, &task.env));

            // Normal arg
            } else {
                args.push(arg.to_owned());
            }
        }

        task.args = args;

        Ok(())
    }

    pub fn expand_deps<F>(&mut self, task: &mut Task, query: F) -> miette::Result<()>
    where
        F: Fn(String) -> miette::Result<Vec<Arc<Project>>>,
    {
        if task.deps.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            deps = ?task.deps,
            "Expanding target scopes for deps",
        );

        let project = &self.project;

        // Dont use a `HashSet` as we want to preserve order
        let mut deps: Vec<Target> = vec![];

        let mut check_and_push_dep = |dep_project: &Project, task_id: &Id| -> miette::Result<()> {
            let Some(dep_task) = dep_project.tasks.get(task_id) else {
                return Err(TasksExpanderError::UnknownTarget {
                    dep: Target::new(&dep_project.id, task_id)?,
                    task: task.target.to_owned(),
                }
                .into());
             };

            // Enforce persistent constraints
            if dep_task.is_persistent() && !task.is_persistent() {
                return Err(TasksExpanderError::PersistentDepRequirement {
                    dep: dep_task.target.to_owned(),
                    task: task.target.to_owned(),
                }
                .into());
            }

            // Add the dep if it has not already been
            let dep = Target::new(&dep_project.id, task_id)?;

            if !deps.contains(&dep) {
                deps.push(dep);
            }

            Ok(())
        };

        for dep_target in &task.deps {
            match &dep_target.scope {
                // :task
                TargetScope::All => {
                    return Err(TasksExpanderError::UnsupportedTargetScopeInDeps {
                        dep: dep_target.to_owned(),
                        task: task.target.to_owned(),
                    }
                    .into());
                }
                // ^:task
                TargetScope::Deps => {
                    let mut dep_ids = project
                        .get_dependency_ids()
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>();

                    // Sort so query cache is more deterministic
                    dep_ids.sort();

                    for dep_project in query(format!("project=[{}]", dep_ids.join(",")))? {
                        check_and_push_dep(&dep_project, &dep_target.task_id)?;
                    }
                }
                // ~:task
                TargetScope::OwnSelf => {
                    if dep_target.task_id == task.id {
                        // Avoid circular references
                    } else {
                        check_and_push_dep(&project, &dep_target.task_id)?;
                    }
                }
                // id:task
                TargetScope::Project(project_id) => {
                    if project_id == &project.id {
                        if dep_target.task_id == task.id {
                            // Avoid circular references
                        } else {
                            check_and_push_dep(&project, &dep_target.task_id)?;
                        }
                    } else {
                        let results = query(format!("project={}", project_id))?;

                        if results.is_empty() {
                            return Err(TasksExpanderError::UnknownTarget {
                                dep: dep_target.to_owned(),
                                task: task.target.to_owned(),
                            }
                            .into());
                        }

                        for dep_project in results {
                            check_and_push_dep(&dep_project, &dep_target.task_id)?;
                        }
                    }
                }
                // #tag:task
                TargetScope::Tag(tag) => {
                    for dep_project in query(format!("tag={tag}"))? {
                        if dep_project.id == project.id {
                            // Avoid circular references
                        } else {
                            check_and_push_dep(&dep_project, &dep_target.task_id)?;
                        }
                    }
                }
            }
        }

        task.deps = deps;

        Ok(())
    }

    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            target = task.target.as_str(),
            env = ?task.env,
            "Expanding environment variables"
        );

        // Substitute environment variables
        let cloned_env = task.env.clone();

        task.env.iter_mut().for_each(|(_, value)| {
            *value = substitute_env_var(value, &cloned_env);
        });

        // Load variables from an .env file
        if let Some(env_file) = &task.options.env_file {
            let env_path = env_file
                .to_workspace_relative(self.project.source.as_str())
                .to_path(self.workspace_root);

            trace!(
                target = task.target.as_str(),
                env_file = ?env_path,
                "Loading environment variables from dotenv",
            );

            // The `.env` file may not have been committed, so avoid crashing
            if env_path.exists() {
                let handle_error = |error: dotenvy::Error| TasksExpanderError::InvalidEnvFile {
                    path: env_path.to_path_buf(),
                    error,
                };

                for line in dotenvy::from_path_iter(&env_path).map_err(handle_error)? {
                    let (key, val) = line.map_err(handle_error)?;

                    // Don't override task-level variables
                    task.env.entry(key).or_insert(val);
                }
            } else {
                warn!(
                    target = task.target.as_str(),
                    env_file = ?env_path,
                    "Setting {} is enabled but file doesn't exist, skipping as this may be intentional",
                    color::symbol("options.envFile"),
                );
            }
        }

        Ok(())
    }

    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.inputs.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            inputs = ?task.inputs,
            "Expanding inputs into file system paths"
        );

        // Expand inputs to file system paths and environment variables
        for input in &task.inputs {
            if let InputPath::EnvVar(var) = input {
                task.input_vars.insert(var.to_owned());
            }
        }

        let (files, globs) =
            TokenExpander::for_inputs(self.project, task, self.workspace_root).expand_inputs()?;

        task.input_paths.extend(files);
        task.input_globs.extend(globs);

        Ok(())
    }

    pub fn expand_outputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            outputs = ?task.outputs,
            "Expanding outputs into file system paths"
        );

        // Expand outputs to file system paths
        let (files, globs) =
            TokenExpander::for_outputs(self.project, task, self.workspace_root).expand_outputs()?;

        // Outputs must *not* be considered an input,
        // so if there's an input that matches an output,
        // remove it! Is there a better way to do this?
        for file in files {
            if task.input_paths.contains(&file) {
                task.input_paths.remove(&file);
            }

            task.output_paths.insert(file);
        }

        for glob in globs {
            if task.input_globs.contains(&glob) {
                task.input_globs.remove(&glob);
            }

            task.output_globs.insert(glob);
        }

        Ok(())
    }
}
