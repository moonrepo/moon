use moon_config::ProjectID;
use moon_logger::{debug, Logable};
use moon_task::{
    ResolverData, Target, TargetError, TargetProjectScope, Task, TaskError, TokenResolver,
};
use moon_utils::{glob, is_ci, path, regex::ENV_VAR};
use rustc_hash::FxHashMap;
use std::path::PathBuf;

use crate::Project;

pub struct TaskExpander<'data> {
    data: &'data ResolverData<'data>,
}

impl<'data> TaskExpander<'data> {
    pub fn new(data: &'data ResolverData) -> Self {
        TaskExpander { data }
    }

    /// Expand the args list to resolve tokens, relative to the project root.
    pub fn expand_args(&self, task: &mut Task) -> Result<(), TaskError> {
        if task.args.is_empty() {
            return Ok(());
        }

        let token_resolver = TokenResolver::for_args(self.data);
        let mut args: Vec<String> = vec![];

        // When running within a project:
        //  - Project paths are relative and start with "./"
        //  - Workspace paths are relative up to the root
        // When running from the workspace:
        //  - All paths are absolute
        let handle_path = |path: PathBuf, is_glob: bool| -> Result<String, TaskError> {
            let arg = if !task.options.run_from_workspace_root
                && path.starts_with(token_resolver.data.workspace_root)
            {
                let rel_path = path::to_string(
                    path::relative_from(&path, token_resolver.data.project_root).unwrap(),
                )?;

                if rel_path.starts_with("..") {
                    rel_path
                } else {
                    format!(".{}{}", std::path::MAIN_SEPARATOR, rel_path)
                }
            } else {
                path::to_string(path)?
            };

            // Annoying, but we need to force forward slashes,
            // and remove drive/UNC prefixes...
            if cfg!(windows) && is_glob {
                return Ok(glob::remove_drive_prefix(path::standardize_separators(arg)));
            }

            Ok(arg)
        };

        // We cant use `TokenResolver.resolve` as args are a mix of strings,
        // strings with tokens, and file paths when tokens are resolved.
        for arg in &task.args {
            if token_resolver.has_token_func(arg) {
                let (paths, globs) = token_resolver.resolve_func(arg, task)?;

                for path in paths {
                    args.push(handle_path(path, false)?);
                }

                for glob in globs {
                    args.push(handle_path(PathBuf::from(glob), true)?);
                }
            } else if token_resolver.has_token_var(arg) {
                args.push(token_resolver.resolve_vars(arg, task)?);
            } else {
                args.push(arg.clone());
            }
        }

        task.args = args;

        Ok(())
    }

    /// Expand the deps list and resolve parent/self scopes.
    pub fn expand_deps(
        &self,
        task: &mut Task,
        owner_id: &str,
        depends_on: &FxHashMap<ProjectID, Project>,
    ) -> Result<(), TaskError> {
        if task.deps.is_empty() {
            return Ok(());
        }

        let mut dep_targets: Vec<Target> = vec![];

        // Dont use a `HashSet` as we want to preserve order
        let mut push_target = |dep: Target| {
            if !dep_targets.contains(&dep) {
                dep_targets.push(dep);
            }
        };

        for target in &task.deps {
            match &target.project {
                // ^:task
                TargetProjectScope::Deps => {
                    for (dep_id, dep_project) in depends_on {
                        if dep_project.tasks.contains_key(&target.task_id) {
                            push_target(Target::new(dep_id, &target.task_id)?);
                        }
                    }
                }
                // ~:task
                TargetProjectScope::OwnSelf => {
                    push_target(Target::new(owner_id, &target.task_id)?);
                }
                // project:task
                TargetProjectScope::Id(_) => {
                    push_target(target.clone());
                }
                _ => {
                    target.fail_with(TargetError::NoProjectAllInTaskDeps(target.id.clone()))?;
                }
            };
        }

        task.deps = dep_targets;

        Ok(())
    }

    /// Expand environment variables by loading a `.env` file if configured.
    pub fn expand_env(&self, task: &mut Task) -> Result<(), TaskError> {
        if let Some(env_file) = &task.options.env_file {
            let env_path = self.data.project_root.join(env_file);
            let error_handler =
                |e: dotenvy::Error| TaskError::InvalidEnvFile(env_path.clone(), e.to_string());

            // The `.env` file may not have been committed, so avoid crashing in CI
            if is_ci() && !env_path.exists() {
                debug!(
                    target: task.get_log_target(),
                    "The `envFile` option is enabled but no `.env` file exists in CI, skipping as this may be intentional",
                );

                return Ok(());
            }

            for entry in dotenvy::from_path_iter(&env_path).map_err(error_handler)? {
                let (key, value) = entry.map_err(error_handler)?;

                // Vars defined in `env` take precedence over those in the env file
                task.env.entry(key).or_insert(value);
            }
        }

        Ok(())
    }

    /// Expand the inputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_inputs(&self, task: &mut Task) -> Result<(), TaskError> {
        if task.inputs.is_empty() {
            return Ok(());
        }

        let token_resolver = TokenResolver::for_inputs(self.data);
        let inputs_without_vars = task
            .inputs
            .clone()
            .into_iter()
            .filter(|i| {
                if ENV_VAR.is_match(i) {
                    task.input_vars.insert(i[1..].to_owned());
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<String>>();

        let (paths, globs) = token_resolver.resolve(&inputs_without_vars, task)?;

        task.input_paths.extend(paths);
        task.input_globs.extend(globs);

        Ok(())
    }

    /// Expand the outputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_outputs(&self, task: &mut Task) -> Result<(), TaskError> {
        if task.outputs.is_empty() {
            return Ok(());
        }

        let token_resolver = TokenResolver::for_outputs(self.data);
        let (paths, globs) = token_resolver.resolve(&task.outputs, task)?;

        task.output_paths.extend(paths);

        if !globs.is_empty() {
            if let Some(glob) = globs.get(0) {
                return Err(TaskError::NoOutputGlob(
                    glob.to_owned(),
                    task.target.id.clone(),
                ));
            }
        }

        Ok(())
    }
}
