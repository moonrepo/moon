use crate::tasks_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::color;
use moon_common::path::{to_virtual_string, WorkspaceRelativePathBuf};
use moon_config::{patterns, InputPath};
use moon_project::Project;
use moon_query::Field;
use moon_task::{Target, TargetScope, Task};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::env;
use std::mem;
use std::path::Path;
use tracing::{trace, warn};

fn substitute_env_var(value: &str, strict: bool) -> String {
    let pattern = if strict {
        &patterns::ENV_VAR_SUBSTITUTE
    } else {
        &patterns::ENV_VAR
    };

    pattern.replace_all(
        value,
        |caps: &patterns::Captures| {
            let name = caps.get(1).unwrap().as_str();
            let value = env::var(name).unwrap_or_default();

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
    pub fn expand<F>(&mut self, query: F) -> miette::Result<()>
    where
        F: Fn(Field) -> miette::Result<Vec<Project>>,
    {
        let mut tasks = BTreeMap::new();

        // Use `mem::take` so that we can mutably borrow the project and tasks in parallel
        for (task_id, mut task) in mem::take(&mut self.project.tasks) {
            // Resolve in this order!
            self.expand_env(&mut task)?;
            self.expand_deps(&mut task, &query)?;
            self.expand_inputs(&mut task)?;
            self.expand_outputs(&mut task)?;
            self.expand_args(&mut task)?;
            self.expand_command(&mut task)?;

            tasks.insert(task_id, task);
        }

        self.project.tasks.extend(tasks);

        Ok(())
    }

    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        // Token variables
        let command =
            TokenExpander::for_command(self.project, task, self.workspace_root).expand_command()?;

        // Environment variables
        task.command = substitute_env_var(&command, false);

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
            } else if patterns::ENV_VAR.is_match(arg) {
                args.push(substitute_env_var(arg, false));

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
        F: Fn(Field) -> miette::Result<Vec<Project>>,
    {
        if task.deps.is_empty() {
            return Ok(());
        }

        let project = &self.project;
        let mut deps: Vec<Target> = vec![];

        // Dont use a `HashSet` as we want to preserve order
        let mut push_dep = |dep: Target| {
            if !deps.contains(&dep) {
                deps.push(dep);
            }
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
                    let dep_ids = project
                        .get_dependency_ids()
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>();

                    for dep_project in query(Field::Project(dep_ids))? {
                        if dep_project.tasks.contains_key(&dep_target.task_id) {
                            push_dep(Target::new(&dep_project.id, &dep_target.task_id)?);
                        }
                    }
                }
                // ~:task
                TargetScope::OwnSelf => {
                    if dep_target.task_id == task.id {
                        // Avoid circular references
                    } else {
                        push_dep(Target::new(&project.id, &dep_target.task_id)?);
                    }
                }
                // id:task
                TargetScope::Project(project_id) => {
                    if project_id == &project.id && dep_target.task_id == task.id {
                        // Avoid circular references
                    } else {
                        push_dep(dep_target.clone());
                    }
                }
                // #tag:task
                TargetScope::Tag(tag) => {
                    for dep_project in query(Field::Tag(vec![tag.to_string()]))? {
                        if dep_project.id == project.id {
                            // Avoid circular references
                        } else {
                            push_dep(Target::new(&dep_project.id, &dep_target.task_id)?);
                        }
                    }
                }
            }
        }

        task.deps = deps;

        Ok(())
    }

    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<()> {
        // Substitute environment variables
        task.env.iter_mut().for_each(|(_, value)| {
            *value = substitute_env_var(value, true);
        });

        // Load variables from an .env file
        if let Some(env_file) = &task.options.env_file {
            let target = &task.target;
            let env_path = env_file
                .to_workspace_relative(self.project.source.as_str())
                .to_path(self.workspace_root);

            trace!(
                target = target.as_str(),
                env_file = ?env_path,
                "Loading env vars from dotenv",
            );

            // The `.env` file may not have been committed, so avoid crashing
            if env_path.exists() {
                let env_vars = dotenvy::from_path_iter(&env_path)
                    .map_err(|error| TasksExpanderError::InvalidEnvFile {
                        path: env_path.to_path_buf(),
                        error,
                    })?
                    .flatten()
                    .collect::<FxHashMap<_, _>>();

                // Don't override task-level variables
                for (key, val) in env_vars {
                    task.env.entry(key).or_insert(val);
                }
            } else {
                warn!(
                    target = target.as_str(),
                    env_file = ?env_path,
                    "The {} option is enabled but file doesn't exist, skipping as this may be intentional",
                    color::id("envFile"),
                );
            }
        }

        Ok(())
    }

    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.inputs.is_empty() {
            return Ok(());
        }

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
