use crate::token_expander::TokenExpander;
use moon_common::color;
use moon_config::TaskArgs;
use moon_env_var::*;
use moon_graph_utils::GraphExpanderContext;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::{Task, TaskFileInput, TaskFileOutput, TaskGlobInput, TaskGlobOutput};
use moon_task_args::parse_task_args;
use std::mem;
use tracing::{debug, instrument, trace, warn};

pub struct TaskExpander<'graph> {
    pub context: &'graph GraphExpanderContext,
    pub token: TokenExpander<'graph>,
    pub project: &'graph Project,
    pub project_graph: &'graph ProjectGraph,
}

impl<'graph> TaskExpander<'graph> {
    pub fn new(
        project_graph: &'graph ProjectGraph,
        project: &'graph Project,
        context: &'graph GraphExpanderContext,
    ) -> Self {
        Self {
            token: TokenExpander::new(project_graph, project, context),
            context,
            project,
            project_graph,
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

            dep.env = EnvSubstitutor::default()
                .with_global_vars(GlobalEnvBag::instance())
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

        task.env = self.token.expand_env(task)?;

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

        task.input_files.extend(result.files_for_input);
        task.input_files.extend(
            result
                .files
                .into_iter()
                .map(|file| (file, TaskFileInput::default())),
        );

        task.input_globs.extend(result.globs_for_input);
        task.input_globs.extend(
            result
                .globs
                .into_iter()
                .map(|glob| (glob, TaskGlobInput::default())),
        );

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

        task.output_files.extend(result.files_for_output);
        task.output_files.extend(
            result
                .files
                .into_iter()
                .map(|file| (file, TaskFileOutput::default())),
        );

        task.output_globs.extend(result.globs_for_output);
        task.output_globs.extend(
            result
                .globs
                .into_iter()
                .map(|glob| (glob, TaskGlobOutput::default())),
        );

        Ok(())
    }

    // Input directories are not allowed, as VCS hashing only operates on files.
    // If we can confirm it's a directory, move it into a glob!
    pub fn move_input_dirs_to_globs(&mut self, task: &mut Task) {
        let mut to_remove = vec![];

        for file in task.input_files.keys() {
            // If this dir is actually an output dir, just remove it
            if task.output_files.contains_key(file) {
                to_remove.push(file.to_owned());
                continue;
            }

            // Otherwise check if it's a dir and not a file
            let abs_file = file.to_path(&self.context.workspace_root);

            if abs_file.exists() && abs_file.is_dir() {
                task.input_globs
                    .insert(file.join("**/*"), TaskGlobInput::default());

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
        for file in task.output_files.keys() {
            task.input_files.remove(file);
        }

        for glob in task.output_globs.keys() {
            task.input_globs.remove(glob);
        }
    }
}
