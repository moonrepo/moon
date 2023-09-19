use crate::expander_context::{substitute_env_var, ExpanderContext, ExpansionBoundaries};
use crate::tasks_expander_error::TasksExpanderError;
use crate::token_expander::TokenExpander;
use moon_common::{color, Id};
use moon_config::InputPath;
use moon_project::Project;
use moon_task::{Target, TargetScope, Task};
use rustc_hash::FxHashSet;
use starbase_utils::glob::GlobSet;
use tracing::{trace, warn};

pub struct TasksExpander<'graph, 'query> {
    pub context: &'graph ExpanderContext<'graph, 'query>,
    pub token: TokenExpander<'graph, 'query>,
}

impl<'graph, 'query> TasksExpander<'graph, 'query> {
    pub fn new(context: &'graph ExpanderContext<'graph, 'query>) -> Self {
        Self {
            token: TokenExpander::new(context),
            context,
        }
    }

    pub fn expand_command(&mut self, task: &mut Task) -> miette::Result<()> {
        trace!(
            target = task.target.as_str(),
            command = &task.command,
            "Expanding tokens and variables in command"
        );

        // Token variables
        let command = self.token.expand_command(task)?;

        // Environment variables
        let command = substitute_env_var(&command, &task.env);

        task.command = command;

        Ok(())
    }

    pub fn expand_args(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.args.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            args = ?task.args,
            "Expanding tokens and variables in args",
        );

        task.args = self.token.expand_args(task)?;

        Ok(())
    }

    pub fn expand_deps(&mut self, task: &mut Task) -> miette::Result<()> {
        if task.deps.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            deps = ?task.deps.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding target scopes for deps",
        );

        let project = &self.context.project;

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

                // Do not depend on tasks that can fail
                if dep_task.options.allow_failure {
                    return Err(TasksExpanderError::AllowFailureDepRequirement {
                        dep: dep_task.target.to_owned(),
                        task: task.target.to_owned(),
                    }
                    .into());
                }

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
                            format!("project={id}", id = dep_ids[0])
                        } else {
                            format!("project=[{ids}]", ids = dep_ids.join(","))
                        };

                        for dep_project in (self.context.query)(input)? {
                            check_and_push_dep(dep_project, &dep_target.task_id, true)?;
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
                        let results =
                            (self.context.query)(format!("project={id}", id = project_id))?;

                        if results.is_empty() {
                            return Err(TasksExpanderError::UnknownTarget {
                                dep: dep_target.to_owned(),
                                task: task.target.to_owned(),
                            }
                            .into());
                        }

                        for dep_project in results {
                            check_and_push_dep(dep_project, &dep_target.task_id, false)?;
                        }
                    }
                }
                // #tag:task
                TargetScope::Tag(tag) => {
                    for dep_project in (self.context.query)(format!("tag={tag}"))? {
                        if dep_project.id == project.id {
                            // Avoid circular references
                        } else {
                            check_and_push_dep(dep_project, &dep_target.task_id, true)?;
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

        // Expand tokens
        let mut env = self.token.expand_env(task)?;
        let cloned_env = env.clone();

        // Substitute environment variables
        for (_, value) in env.iter_mut() {
            *value = substitute_env_var(value, &cloned_env);
        }

        // Load variables from an .env file
        if let Some(env_file) = &task.options.env_file {
            let env_path = env_file
                .to_workspace_relative(self.context.project.source.as_str())
                .to_path(self.context.workspace_root);

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
                    color::property("options.envFile"),
                );
            }
        }

        task.env = env;

        Ok(())
    }

    pub fn expand_inputs(&mut self, task: &mut Task) -> miette::Result<()> {
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

        let (files, globs) = self.token.expand_inputs(task)?;

        task.input_vars.extend(vars);
        task.input_files.extend(files);
        task.input_globs.extend(globs);

        Ok(())
    }

    pub fn expand_outputs(
        &mut self,
        task: &mut Task,
        boundaries: &mut ExpansionBoundaries,
    ) -> miette::Result<()> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        trace!(
            target = task.target.as_str(),
            outputs = ?task.outputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
            "Expanding outputs into file system paths"
        );

        // Expand outputs to file system paths
        let (files, globs) = self.token.expand_outputs(task)?;

        // Aggregate paths first before globbing, as they are literal
        for file in files {
            if self.context.check_boundaries {
                if let Some(existing_target) = boundaries.output_files.get(&file) {
                    return Err(TasksExpanderError::OverlappingOutputs {
                        output: file.to_string(),
                        targets: vec![existing_target.to_owned(), task.target.clone()],
                    }
                    .into());
                } else {
                    boundaries
                        .output_files
                        .insert(file.clone(), task.target.clone());
                }
            }

            // Outputs must *not* be considered an input,
            // so if there's an input that matches an output,
            // remove it! Is there a better way to do this?
            if task.input_files.contains(&file) {
                task.input_files.remove(&file);
            }

            task.output_files.insert(file);
        }

        // Aggregate globs second so we can match against the paths
        for glob in globs {
            if self.context.check_boundaries {
                if let Some(existing_target) = boundaries.output_globs.get(&glob) {
                    return Err(TasksExpanderError::OverlappingOutputs {
                        output: glob.to_string(),
                        targets: vec![existing_target.to_owned(), task.target.clone()],
                    }
                    .into());
                } else {
                    boundaries
                        .output_globs
                        .insert(glob.clone(), task.target.clone());
                }
            }

            if task.input_globs.contains(&glob) {
                task.input_globs.remove(&glob);
            }

            task.output_globs.insert(glob);
        }

        // Now that we have globs, match against all aggreated paths for boundary conflicts,
        // primarily overlapping outputs!
        if self.context.check_boundaries
            && !task.output_globs.is_empty()
            && !boundaries.output_files.is_empty()
        {
            let globset = GlobSet::new(&task.output_globs)?;
            let globs_without_stars = task
                .output_globs
                .iter()
                .filter_map(|g| g.as_str().strip_suffix("/**/*"))
                .collect::<FxHashSet<_>>();

            for (existing_file, existing_target) in &boundaries.output_files {
                if existing_target != &task.target
                    && (globset.is_match(existing_file.as_str())
                        || globs_without_stars.contains(existing_file.as_str()))
                {
                    return Err(TasksExpanderError::OverlappingOutputs {
                        output: existing_file.to_string(),
                        targets: vec![existing_target.to_owned(), task.target.clone()],
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}
