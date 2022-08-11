use crate::errors::{TargetError, TaskError};
use crate::target::{Target, TargetProjectScope};
use crate::token::{ResolverData, TokenResolver};
use crate::types::{EnvVars, TouchedFilePaths};
use moon_config::{
    DependencyConfig, FileGlob, FilePath, InputValue, PlatformType, TargetID, TaskConfig,
    TaskMergeStrategy, TaskOptionEnvFile, TaskOptionsConfig, TaskOutputStyle,
};
use moon_logger::{color, debug, trace, Logable};
use moon_utils::{glob, path, regex::ENV_VAR, string_vec};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptions {
    pub cache: bool,

    pub env_file: Option<String>,

    pub merge_args: TaskMergeStrategy,

    pub merge_deps: TaskMergeStrategy,

    pub merge_env: TaskMergeStrategy,

    pub merge_inputs: TaskMergeStrategy,

    pub merge_outputs: TaskMergeStrategy,

    pub output_style: Option<TaskOutputStyle>,

    pub retry_count: u8,

    pub run_deps_in_parallel: bool,

    pub run_in_ci: bool,

    pub run_from_workspace_root: bool,
}

impl Default for TaskOptions {
    fn default() -> Self {
        TaskOptions {
            cache: true,
            env_file: None,
            merge_args: TaskMergeStrategy::Append,
            merge_deps: TaskMergeStrategy::Append,
            merge_env: TaskMergeStrategy::Append,
            merge_inputs: TaskMergeStrategy::Append,
            merge_outputs: TaskMergeStrategy::Append,
            output_style: None,
            retry_count: 0,
            run_deps_in_parallel: true,
            run_in_ci: true,
            run_from_workspace_root: false,
        }
    }
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
        if let Some(env_file) = &config.env_file {
            self.env_file = env_file.to_option();
        }

        if let Some(merge_args) = &config.merge_args {
            self.merge_args = merge_args.clone();
        }

        if let Some(merge_deps) = &config.merge_deps {
            self.merge_deps = merge_deps.clone();
        }

        if let Some(merge_env) = &config.merge_env {
            self.merge_env = merge_env.clone();
        }

        if let Some(merge_inputs) = &config.merge_inputs {
            self.merge_inputs = merge_inputs.clone();
        }

        if let Some(merge_outputs) = &config.merge_outputs {
            self.merge_outputs = merge_outputs.clone();
        }

        if let Some(output_style) = &config.output_style {
            self.output_style = Some(output_style.clone());
        }

        if let Some(retry_count) = &config.retry_count {
            self.retry_count = *retry_count;
        }

        if let Some(run_deps_in_parallel) = &config.run_deps_in_parallel {
            self.run_deps_in_parallel = *run_deps_in_parallel;
        }

        if let Some(run_in_ci) = &config.run_in_ci {
            self.run_in_ci = *run_in_ci;
        }

        if let Some(run_from_workspace_root) = &config.run_from_workspace_root {
            self.run_from_workspace_root = *run_from_workspace_root;
        }
    }

    pub fn to_config(&self) -> TaskOptionsConfig {
        let default_options = TaskOptions::default();
        let mut config = TaskOptionsConfig::default();

        // Skip merge options until we need them

        if let Some(env_file) = &self.env_file {
            config.env_file = Some(if env_file == ".env" {
                TaskOptionEnvFile::Enabled(true)
            } else {
                TaskOptionEnvFile::File(env_file.clone())
            });
        }

        if let Some(output_style) = &self.output_style {
            config.output_style = Some(output_style.clone());
        }

        if self.run_deps_in_parallel != default_options.run_deps_in_parallel {
            config.run_deps_in_parallel = Some(self.run_deps_in_parallel);
        }

        if self.retry_count != default_options.retry_count {
            config.retry_count = Some(self.retry_count);
        }

        if self.run_in_ci != default_options.run_in_ci {
            config.run_in_ci = Some(self.run_in_ci);
        }

        if self.run_from_workspace_root != default_options.run_from_workspace_root {
            config.run_from_workspace_root = Some(self.run_from_workspace_root);
        }

        config
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub args: Vec<String>,

    pub command: String,

    pub deps: Vec<TargetID>,

    pub env: EnvVars,

    pub inputs: Vec<InputValue>,

    pub input_globs: HashSet<FileGlob>,

    pub input_paths: HashSet<PathBuf>,

    pub input_vars: HashSet<String>,

    #[serde(skip)]
    pub log_target: String,

    pub options: TaskOptions,

    pub outputs: Vec<FilePath>,

    pub output_paths: HashSet<PathBuf>,

    pub platform: PlatformType,

    pub target: TargetID,
}

impl Logable for Task {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

impl Task {
    pub fn new<T: AsRef<str>>(target: T) -> Self {
        let target = target.as_ref();
        let log_target = format!("moon:project:{}", target);

        Task {
            inputs: string_vec!["**/*"],
            log_target,
            target: target.to_owned(),
            ..Task::default()
        }
    }

    pub fn from_config(target: TargetID, config: &TaskConfig) -> Self {
        let cloned_config = config.clone();
        let cloned_options = cloned_config.options;
        let command = cloned_config.command.unwrap_or_default();
        let is_local =
            cloned_config.local || command == "dev" || command == "serve" || command == "start";
        let log_target = format!("moon:project:{}", target);

        let task = Task {
            args: cloned_config.args.unwrap_or_default(),
            command,
            deps: cloned_config.deps.unwrap_or_default(),
            env: cloned_config.env.unwrap_or_default(),
            inputs: cloned_config.inputs.unwrap_or_else(|| string_vec!["**/*"]),
            input_vars: HashSet::new(),
            input_globs: HashSet::new(),
            input_paths: HashSet::new(),
            log_target,
            options: TaskOptions {
                cache: cloned_options.cache.unwrap_or(!is_local),
                env_file: cloned_options
                    .env_file
                    .map(|env_file| env_file.to_option().unwrap()),
                merge_args: cloned_options.merge_args.unwrap_or_default(),
                merge_deps: cloned_options.merge_deps.unwrap_or_default(),
                merge_env: cloned_options.merge_env.unwrap_or_default(),
                merge_inputs: cloned_options.merge_inputs.unwrap_or_default(),
                merge_outputs: cloned_options.merge_outputs.unwrap_or_default(),
                output_style: match cloned_options.output_style {
                    Some(style) => Some(style),
                    None => {
                        if is_local {
                            Some(TaskOutputStyle::Stream)
                        } else {
                            None
                        }
                    }
                },
                retry_count: cloned_options.retry_count.unwrap_or_default(),
                run_deps_in_parallel: cloned_options.run_deps_in_parallel.unwrap_or(true),
                run_in_ci: cloned_options.run_in_ci.unwrap_or(!is_local),
                run_from_workspace_root: cloned_options.run_from_workspace_root.unwrap_or_default(),
            },
            outputs: cloned_config.outputs.unwrap_or_default(),
            output_paths: HashSet::new(),
            platform: cloned_config.type_of,
            target: target.clone(),
        };

        debug!(
            target: &task.log_target,
            "Creating task {} with command {}",
            color::target(&target),
            color::shell(&task.command)
        );

        task
    }

    pub fn to_config(&self) -> TaskConfig {
        let mut config = TaskConfig {
            command: Some(self.command.clone()),
            options: self.options.to_config(),
            ..TaskConfig::default()
        };

        if !self.args.is_empty() {
            config.args = Some(self.args.clone());
        }

        if !self.deps.is_empty() {
            config.deps = Some(self.deps.clone());
        }

        if !self.env.is_empty() {
            config.env = Some(self.env.clone());
        }

        if !self.inputs.is_empty()
            || (self.inputs.len() == 1 && !self.inputs.contains(&"**/*".to_owned()))
        {
            config.inputs = Some(self.inputs.clone());
        }

        if !self.outputs.is_empty() {
            config.outputs = Some(self.outputs.clone());
        }

        if !matches!(self.platform, PlatformType::Unknown) {
            config.type_of = self.platform.clone();
        }

        config
    }

    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> Result<glob::GlobSet, TaskError> {
        Ok(glob::GlobSet::new(
            self.input_globs
                .iter()
                .map(|g| {
                    if cfg!(windows) {
                        glob::remove_drive_prefix(g)
                    } else {
                        g.to_owned()
                    }
                })
                .collect::<Vec<String>>(),
        )?)
    }

    /// Expand the args list to resolve tokens, relative to the project root.
    pub fn expand_args(&mut self, token_resolver: TokenResolver) -> Result<(), TaskError> {
        if self.args.is_empty() {
            return Ok(());
        }

        let mut args: Vec<String> = vec![];

        // When running within a project:
        //  - Project paths are relative and start with "./"
        //  - Workspace paths are relative up to the root
        // When running from the workspace:
        //  - All paths are absolute
        let handle_path = |path: PathBuf, is_glob: bool| -> Result<String, TaskError> {
            let arg = if !self.options.run_from_workspace_root
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
        for arg in &self.args {
            if token_resolver.has_token_func(arg) {
                let (paths, globs) = token_resolver.resolve_func(arg, self)?;

                for path in paths {
                    args.push(handle_path(path, false)?);
                }

                for glob in globs {
                    args.push(handle_path(PathBuf::from(glob), true)?);
                }
            } else if token_resolver.has_token_var(arg) {
                args.push(token_resolver.resolve_vars(arg, self)?);
            } else {
                args.push(arg.clone());
            }
        }

        self.args = args;

        Ok(())
    }

    /// Expand the deps list and resolve parent/self scopes.
    pub fn expand_deps(
        &mut self,
        owner_id: &str,
        depends_on: &[DependencyConfig],
    ) -> Result<(), TaskError> {
        if self.deps.is_empty() {
            return Ok(());
        }

        let mut deps: Vec<String> = vec![];

        // Dont use a `HashSet` as we want to preserve order
        let mut push_dep = |dep: String| {
            if !deps.contains(&dep) {
                deps.push(dep);
            }
        };

        for dep in &self.deps {
            let target = Target::parse(dep)?;

            match &target.project {
                // ^:task
                TargetProjectScope::Deps => {
                    for dep_cfg in depends_on {
                        push_dep(Target::format(&dep_cfg.id, &target.task_id)?);
                    }
                }
                // ~:task
                TargetProjectScope::Own => {
                    push_dep(Target::format(owner_id, &target.task_id)?);
                }
                // project:task
                TargetProjectScope::Id(_) => {
                    push_dep(dep.clone());
                }
                _ => {
                    target.fail_with(TargetError::NoProjectAllInTaskDeps(target.id.clone()))?;
                }
            };
        }

        self.deps = deps;

        Ok(())
    }

    /// Expand environment variables by loading a `.env` file if configured.
    pub fn expand_env(&mut self, data: &ResolverData) -> Result<(), TaskError> {
        if let Some(env_file) = &self.options.env_file {
            let env_path = data.project_root.join(env_file);
            let error_handler =
                |e: dotenvy::Error| TaskError::InvalidEnvFile(env_path.clone(), e.to_string());

            for entry in dotenvy::from_path_iter(&env_path).map_err(error_handler)? {
                let (key, value) = entry.map_err(error_handler)?;

                // Vars defined in `env` take precedence over those in the env file
                self.env.entry(key).or_insert(value);
            }
        }

        Ok(())
    }

    /// Expand the inputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_inputs(&mut self, token_resolver: TokenResolver) -> Result<(), TaskError> {
        if self.inputs.is_empty() {
            return Ok(());
        }

        let inputs_without_vars = self
            .inputs
            .clone()
            .into_iter()
            .filter(|i| {
                if ENV_VAR.is_match(i) {
                    self.input_vars.insert(i[1..].to_owned());
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<String>>();

        let (paths, globs) = token_resolver.resolve(&inputs_without_vars, self)?;

        self.input_paths.extend(paths);
        self.input_globs.extend(globs);

        Ok(())
    }

    /// Expand the outputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_outputs(&mut self, token_resolver: TokenResolver) -> Result<(), TaskError> {
        if self.outputs.is_empty() {
            return Ok(());
        }

        let (paths, globs) = token_resolver.resolve(&self.outputs, self)?;

        self.output_paths.extend(paths);

        if !globs.is_empty() {
            if let Some(glob) = globs.get(0) {
                return Err(TaskError::NoOutputGlob(
                    glob.to_owned(),
                    self.target.clone(),
                ));
            }
        }

        Ok(())
    }

    /// Return true if this task is affected, based on touched files.
    /// Will attempt to find any file that matches our list of inputs.
    pub fn is_affected(&self, touched_files: &TouchedFilePaths) -> Result<bool, TaskError> {
        for var_name in &self.input_vars {
            if let Ok(var) = env::var(var_name) {
                if !var.is_empty() {
                    trace!(
                        target: self.get_log_target(),
                        "Affected by {} (via environment variable)",
                        color::symbol(var_name),
                    );

                    return Ok(true);
                }
            }
        }

        let has_globs = !self.input_globs.is_empty();
        let globset = self.create_globset()?;

        for file in touched_files {
            if self.input_paths.contains(file) {
                trace!(
                    target: self.get_log_target(),
                    "Affected by {} (via input files)",
                    color::path(file),
                );

                return Ok(true);
            }

            if has_globs && globset.matches(file)? {
                trace!(
                    target: self.get_log_target(),
                    "Affected by {} (via input globs)",
                    color::path(file),
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return true if the task is a "no operation" and does nothing.
    pub fn is_no_op(&self) -> bool {
        self.command == "nop" || self.command == "noop" || self.command == "no-op"
    }

    pub fn merge(&mut self, config: &TaskConfig) {
        // Merge options first incase the merge strategy has changed
        self.options.merge(&config.options);
        self.platform = config.type_of.clone();

        // Then merge the actual task fields
        if let Some(command) = &config.command {
            self.command = command.clone();
        }

        if let Some(args) = &config.args {
            self.args = self.merge_string_vec(&self.args, args, &self.options.merge_args);
        }

        if let Some(deps) = &config.deps {
            self.deps = self.merge_string_vec(&self.deps, deps, &self.options.merge_deps);
        }

        if let Some(env) = &config.env {
            self.env = self.merge_env_vars(&self.env, env, &self.options.merge_env);
        }

        if let Some(inputs) = &config.inputs {
            self.inputs = self.merge_string_vec(&self.inputs, inputs, &self.options.merge_inputs);
        }

        if let Some(outputs) = &config.outputs {
            self.outputs =
                self.merge_string_vec(&self.outputs, outputs, &self.options.merge_outputs);
        }
    }

    pub fn should_run_in_ci(&self) -> bool {
        !self.outputs.is_empty() || self.options.run_in_ci
    }

    fn merge_env_vars(
        &self,
        base: &EnvVars,
        next: &EnvVars,
        strategy: &TaskMergeStrategy,
    ) -> EnvVars {
        match strategy {
            TaskMergeStrategy::Append => {
                let mut map = base.clone();
                map.extend(next.clone());
                map
            }
            TaskMergeStrategy::Prepend => {
                let mut map = next.clone();
                map.extend(base.clone());
                map
            }
            TaskMergeStrategy::Replace => next.clone(),
        }
    }

    fn merge_string_vec(
        &self,
        base: &[String],
        next: &[String],
        strategy: &TaskMergeStrategy,
    ) -> Vec<String> {
        let mut list: Vec<String> = vec![];

        // This is easier than .extend() as we need to clone the inner string
        let mut merge = |inner_list: &[String]| {
            for item in inner_list {
                list.push(item.clone());
            }
        };

        match strategy {
            TaskMergeStrategy::Append => {
                merge(base);
                merge(next);
            }
            TaskMergeStrategy::Prepend => {
                merge(next);
                merge(base);
            }
            TaskMergeStrategy::Replace => {
                merge(next);
            }
        }

        list
    }
}
