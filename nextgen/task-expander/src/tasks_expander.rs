use crate::tasks_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::path::{to_virtual_string, WorkspaceRelativePathBuf};
use moon_common::{color, Id};
use moon_config::{patterns, InputPath};
use moon_project::Project;
use moon_task::{Target, TargetScope, Task};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::path::Path;
use tracing::{trace, warn};

fn substitute_env_var(value: &str, env_map: &FxHashMap<String, String>) -> String {
    patterns::ENV_VAR_SUBSTITUTE.replace_all(
        value,
        |caps: &patterns::Captures| {
            // First with wrapping {}, then without
            let name = caps.get(1).or_else(|| caps.get(2)).unwrap().as_str();

            match env_map.get(name).map(|v| v.to_owned()).or_else(|| env::var(name).ok()) {
                Some(var) => var,
                None => {
                     warn!(
                        "Task value `{}` contains the environment variable ${}, but this variable is not set. Not substituting and keeping as-is.",
                        value,
                        name
                    );

                    caps.get(0).unwrap().as_str().to_owned()
                }
            }
        })
    .to_string()
}

pub struct TasksExpander<'proj> {
    pub project: &'proj mut Project,
    pub workspace_root: &'proj Path,
}

impl<'proj> TasksExpander<'proj> {
    pub fn expand<F>(
        project: &'proj mut Project,
        workspace_root: &'proj Path,
        query: F,
    ) -> miette::Result<()>
    where
        F: Fn(String) -> miette::Result<Vec<&'proj Project>>,
    {
        // We unfortunately need to clone the keys here since we can't
        // borrow the project mutably (for the expander) while we're
        // also iterating over and mutating the tasks.
        let task_ids = project.tasks.keys().cloned().collect::<Vec<_>>();

        let mut expander = TasksExpander {
            project,
            workspace_root,
        };

        for task_id in &task_ids {
            // Resolve in this order!
            expander.expand_env(task_id)?;
            expander.expand_deps(task_id, &query)?;
            expander.expand_inputs(task_id)?;
            expander.expand_outputs(task_id)?;
            expander.expand_args(task_id)?;
            expander.expand_command(task_id)?;
        }

        Ok(())
    }

    pub fn expand_command(&mut self, task_id: &str) -> miette::Result<()> {
        let task = self.get_task(task_id);

        trace!(
            target = task.target.as_str(),
            command = &task.command,
            "Expanding tokens and variables in command"
        );

        // Token variables
        let command =
            TokenExpander::for_command(self.project, task, self.workspace_root).expand_command()?;

        // Environment variables
        let command = substitute_env_var(&command, &task.env);

        {
            self.get_task_mut(task_id).command = command;
        }

        Ok(())
    }

    pub fn expand_args(&mut self, task_id: &str) -> miette::Result<()> {
        let task = self.get_task(task_id);

        if task.args.is_empty() {
            return Ok(());
        }

        let handle_path = |path: WorkspaceRelativePathBuf| -> miette::Result<String> {
            if task.options.run_from_workspace_root {
                Ok(format!("./{}", path))
            } else if let Ok(proj_path) = path.strip_prefix(&self.project.source) {
                Ok(format!("./{}", proj_path))
            } else {
                // TODO
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

        {
            self.get_task_mut(task_id).args = args;
        }

        Ok(())
    }

    pub fn expand_deps<F>(&mut self, task_id: &str, query: F) -> miette::Result<()>
    where
        F: Fn(String) -> miette::Result<Vec<&'proj Project>>,
    {
        let task = self.get_task(task_id);

        if task.deps.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            deps = ?task.deps.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding target scopes for deps",
        );

        let project = &self.project;

        // Dont use a `HashSet` as we want to preserve order
        let mut deps: Vec<Target> = vec![];

        let mut check_and_push_dep =
            |dep_project: &Project, task_id: &Id, skip_if_missing: bool| -> miette::Result<()> {
                let Some(dep_task) = dep_project.tasks.get(task_id) else {
                    if skip_if_missing {
                        return Ok(());
                    }

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

                    if !dep_ids.is_empty() {
                        // Sort so query cache is more deterministic
                        dep_ids.sort();

                        let input = if dep_ids.len() == 1 {
                            format!("project={id} || projectAlias={id}", id = dep_ids.join(""))
                        } else {
                            format!(
                                "project=[{ids}] || projectAlias=[{ids}]",
                                ids = dep_ids.join(",")
                            )
                        };

                        for dep_project in query(input)? {
                            check_and_push_dep(&dep_project, &dep_target.task_id, true)?;
                        }
                    }
                }
                // ~:task
                TargetScope::OwnSelf => {
                    if dep_target.task_id == task.id {
                        // Avoid circular references
                    } else {
                        check_and_push_dep(project, &dep_target.task_id, false)?;
                    }
                }
                // id:task
                TargetScope::Project(project_id) => {
                    if project_id == &project.id {
                        if dep_target.task_id == task.id {
                            // Avoid circular references
                        } else {
                            check_and_push_dep(project, &dep_target.task_id, false)?;
                        }
                    } else {
                        let results = query(format!(
                            "project={id} || projectAlias={id}",
                            id = project_id
                        ))?;

                        if results.is_empty() {
                            return Err(TasksExpanderError::UnknownTarget {
                                dep: dep_target.to_owned(),
                                task: task.target.to_owned(),
                            }
                            .into());
                        }

                        for dep_project in results {
                            check_and_push_dep(&dep_project, &dep_target.task_id, false)?;
                        }
                    }
                }
                // #tag:task
                TargetScope::Tag(tag) => {
                    for dep_project in query(format!("tag={tag}"))? {
                        if dep_project.id == project.id {
                            // Avoid circular references
                        } else {
                            check_and_push_dep(&dep_project, &dep_target.task_id, true)?;
                        }
                    }
                }
            }
        }

        {
            self.get_task_mut(task_id).deps = deps;
        }

        Ok(())
    }

    pub fn expand_env(&mut self, task_id: &str) -> miette::Result<()> {
        let task = self.get_task(task_id);

        trace!(
            target = task.target.as_str(),
            env = ?task.env,
            "Expanding environment variables"
        );

        // Substitute environment variables
        let cloned_env = task.env.clone();
        let mut env = FxHashMap::default();

        for (key, val) in &task.env {
            env.insert(key.to_owned(), substitute_env_var(val, &cloned_env));
        }

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
                    env.entry(key).or_insert(val);
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

        {
            self.get_task_mut(task_id).env = env;
        }

        Ok(())
    }

    pub fn expand_inputs(&mut self, task_id: &str) -> miette::Result<()> {
        let task = self.get_task(task_id);

        if task.inputs.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            inputs = ?task.inputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding inputs into file system paths"
        );

        // Expand inputs to file system paths and environment variables
        let mut vars = FxHashSet::default();

        for input in &task.inputs {
            if let InputPath::EnvVar(var) = input {
                vars.insert(var.to_owned());
            }
        }

        let (files, globs) =
            TokenExpander::for_inputs(self.project, task, self.workspace_root).expand_inputs()?;

        {
            let task = self.get_task_mut(task_id);
            task.input_vars.extend(vars);
            task.input_paths.extend(files);
            task.input_globs.extend(globs);
        }

        Ok(())
    }

    pub fn expand_outputs(&mut self, task_id: &str) -> miette::Result<()> {
        let task = self.get_task(task_id);

        if task.outputs.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            outputs = ?task.outputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding outputs into file system paths"
        );

        // Expand outputs to file system paths
        let (files, globs) =
            TokenExpander::for_outputs(self.project, task, self.workspace_root).expand_outputs()?;

        // Outputs must *not* be considered an input,
        // so if there's an input that matches an output,
        // remove it! Is there a better way to do this?
        {
            let task = self.get_task_mut(task_id);

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
        }

        Ok(())
    }

    fn get_task(&self, task_id: &str) -> &Task {
        self.project.tasks.get(task_id).unwrap()
    }

    fn get_task_mut(&mut self, task_id: &str) -> &mut Task {
        self.project.tasks.get_mut(task_id).unwrap()
    }
}
