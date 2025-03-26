use crate::task_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::color;
use moon_config::TaskArgs;
use moon_env_var::*;
use moon_graph_utils::GraphExpanderContext;
use moon_project::Project;
use moon_task::Task;
use moon_task_args::parse_task_args;
use rustc_hash::FxHashMap;
use std::mem;
use tracing::{debug, instrument, trace, warn};

pub struct TaskExpander<'graph> {
    pub context: &'graph GraphExpanderContext,
    pub token: TokenExpander<'graph>,
    pub project: &'graph Project,
}

impl<'graph> TaskExpander<'graph> {
    pub fn new(project: &'graph Project, context: &'graph GraphExpanderContext) -> Self {
        Self {
            token: TokenExpander::new(project, context),
            context,
            project,
        }
    }

    #[instrument(name = "expand_task", skip_all)]
    pub fn expand(mut self, task: &Task) -> miette::Result<Task> {
        let mut task = task.to_owned();

        debug!(
            task_target = task.target.as_str(),
            "Expanding task {}",
            color::label(&task.target)
        );

        // Resolve in this order!
        if task.script.is_some() {
            self.expand_script(&mut task)?;
        } else {
            self.expand_command(&mut task)?;
        }

        self.expand_env(&mut task)?;
        self.expand_deps(&mut task)?;
        self.expand_inputs(&mut task)?;
        self.expand_outputs(&mut task)?;
        self.expand_args(&mut task)?;
        task.state.expanded = true;

        // Run post-expand operations
        self.move_input_dirs_to_globs(&mut task);
        self.remove_input_output_overlaps(&mut task);

        Ok(task)
    }

    #[instrument(skip_all)]
    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            task_target = task.target.as_str(),
            command = &task.command,
            "Expanding tokens and variables in command"
        );

        task.command = self.token.expand_command(task)?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_script(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            task_target = task.target.as_str(),
            script = task.script.as_ref(),
            "Expanding tokens and variables in script"
        );

        task.script = Some(self.token.expand_script(task)?);

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.args.is_empty() {
            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            args = ?task.args,
            "Expanding tokens and variables in args",
        );

        task.args = self.token.expand_args(task)?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_deps(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.deps.is_empty() {
            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            deps = ?task.deps.iter().map(|d| d.target.as_str()).collect::<Vec<_>>(),
            "Expanding tokens and variables in deps args and env",
        );

        let mut deps = mem::take(&mut task.deps);

        for dep in deps.iter_mut() {
            let dep_args = self
                .token
                .expand_args_with_task(task, Some(parse_task_args(&dep.args)?))?;
            let dep_env = self
                .token
                .expand_env_with_task(task, Some(mem::take(&mut dep.env)))?;

            dep.args = if dep_args.is_empty() {
                TaskArgs::None
            } else {
                TaskArgs::List(dep_args)
            };

            dep.env = EnvSubstitutor::new()
                .with_local_vars(&dep_env)
                .substitute_all(&dep_env);
        }

        task.deps = deps;

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_env(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            task_target = task.target.as_str(),
            env = ?task.env,
            "Expanding environment variables"
        );

        let mut env = self.token.expand_env(task)?;

        // Load variables from an .env file
        if let Some(env_files) = &task.options.env_files {
            let env_paths = env_files
                .iter()
                .map(|file| {
                    file.to_workspace_relative(self.project.source.as_str())
                        .to_path(&self.context.workspace_root)
                })
                .collect::<Vec<_>>();

            trace!(
                task_target = task.target.as_str(),
                env_files = ?env_paths,
                "Loading environment variables from .env files",
            );

            let mut merged_env_vars = FxHashMap::default();

            // The file may not have been committed, so avoid crashing
            for env_path in env_paths {
                if env_path.exists() {
                    trace!(
                        task_target = task.target.as_str(),
                        env_file = ?env_path,
                        "Loading .env file",
                    );

                    let handle_error = |error: dotenvy::Error| TasksExpanderError::InvalidEnvFile {
                        path: env_path.to_path_buf(),
                        error: Box::new(error),
                    };

                    for line in dotenvy::from_path_iter(&env_path).map_err(handle_error)? {
                        let (key, val) = line.map_err(handle_error)?;

                        // Overwrite previous values
                        merged_env_vars.insert(key, val);
                    }
                } else {
                    trace!(
                        task_target = task.target.as_str(),
                        env_file = ?env_path,
                        "Skipping .env file because it doesn't exist",
                    );
                }
            }

            // Don't override task-level variables
            for (key, val) in merged_env_vars {
                env.entry(key).or_insert(val);
            }
        }

        task.env = EnvSubstitutor::new()
            .with_local_vars(&env)
            .substitute_all(&env);

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.inputs.is_empty() {
            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            inputs = ?task.inputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding inputs into file system paths"
        );

        // Expand inputs to file system paths and environment variables
        let result = self.token.expand_inputs(task)?;

        task.input_env.extend(result.env);
        task.input_files.extend(result.files);
        task.input_globs.extend(result.globs);

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn expand_outputs(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        trace!(
            task_target = task.target.as_str(),
            outputs = ?task.outputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding outputs into file system paths"
        );

        // Expand outputs to file system paths
        let result = self.token.expand_outputs(task)?;

        task.output_files.extend(result.files);
        task.output_globs.extend(result.globs);

        Ok(())
    }

    // Input directories are not allowed, as VCS hashing only operates on files.
    // If we can confirm it's a directory, move it into a glob!
    pub fn move_input_dirs_to_globs(&mut self, task: &mut Task) {
        let mut to_remove = vec![];

        for file in &task.input_files {
            // If this dir is actually an output dir, just remove it
            if task.output_files.contains(file) {
                to_remove.push(file.to_owned());
                continue;
            }

            // Otherwise check if it's a dir and not a file
            let abs_file = file.to_path(&self.context.workspace_root);

            if abs_file.exists() && abs_file.is_dir() {
                task.input_globs.insert(file.join("**/*"));
                to_remove.push(file.to_owned());
            }
        }

        for file in to_remove {
            task.input_files.remove(&file);
        }
    }

    // Outputs must not be considered an input, otherwise the content
    // hash will constantly change, and the cache will always be busted
    pub fn remove_input_output_overlaps(&mut self, task: &mut Task) {
        for file in &task.output_files {
            task.input_files.remove(file);
        }

        for glob in &task.output_globs {
            task.input_globs.remove(glob);
        }
    }
}
