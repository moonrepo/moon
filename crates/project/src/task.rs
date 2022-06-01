use crate::errors::{ProjectError, TargetError};
use crate::target::{Target, TargetProject};
use crate::token::TokenResolver;
use crate::types::{EnvVars, ExpandedFiles, TouchedFilePaths};
use moon_config::{
    FilePath, FilePathOrGlob, TargetID, TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType,
};
use moon_logger::{color, debug, map_list, trace, Logable};
use moon_utils::{glob, path, string_vec};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptions {
    pub merge_args: TaskMergeStrategy,

    pub merge_deps: TaskMergeStrategy,

    pub merge_env: TaskMergeStrategy,

    pub merge_inputs: TaskMergeStrategy,

    pub merge_outputs: TaskMergeStrategy,

    pub retry_count: u8,

    pub run_in_ci: bool,

    pub run_from_workspace_root: bool,
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
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

        if let Some(retry_count) = &config.retry_count {
            self.retry_count = *retry_count;
        }

        if let Some(run_in_ci) = &config.run_in_ci {
            self.run_in_ci = *run_in_ci;
        }

        if let Some(run_from_workspace_root) = &config.run_from_workspace_root {
            self.run_from_workspace_root = *run_from_workspace_root;
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub args: Vec<String>,

    pub command: String,

    pub deps: Vec<TargetID>,

    pub env: EnvVars,

    pub inputs: Vec<FilePathOrGlob>,

    pub input_globs: Vec<FilePathOrGlob>,

    pub input_paths: ExpandedFiles,

    #[serde(skip)]
    pub log_target: String,

    pub options: TaskOptions,

    pub outputs: Vec<FilePath>,

    pub output_paths: ExpandedFiles,

    pub target: TargetID,

    #[serde(rename = "type")]
    pub type_of: TaskType,
}

impl Logable for Task {
    fn get_log_target(&self) -> &str {
        &self.log_target
    }
}

impl Task {
    pub fn from_config(target: TargetID, config: &TaskConfig) -> Self {
        let cloned_config = config.clone();
        let cloned_options = cloned_config.options;
        let command = cloned_config.command.unwrap_or_default();
        let is_long_running = command == "serve" || command == "start";
        let log_target = format!("moon:project:{}", target);

        let task = Task {
            args: cloned_config.args.unwrap_or_default(),
            command,
            deps: cloned_config.deps.unwrap_or_default(),
            env: cloned_config.env.unwrap_or_default(),
            inputs: cloned_config.inputs.unwrap_or_else(|| string_vec!["**/*"]),
            input_globs: vec![],
            input_paths: HashSet::new(),
            log_target,
            options: TaskOptions {
                merge_args: cloned_options.merge_args.unwrap_or_default(),
                merge_deps: cloned_options.merge_deps.unwrap_or_default(),
                merge_env: cloned_options.merge_env.unwrap_or_default(),
                merge_inputs: cloned_options.merge_inputs.unwrap_or_default(),
                merge_outputs: cloned_options.merge_outputs.unwrap_or_default(),
                retry_count: cloned_options.retry_count.unwrap_or_default(),
                run_in_ci: cloned_options.run_in_ci.unwrap_or(!is_long_running),
                run_from_workspace_root: cloned_options.run_from_workspace_root.unwrap_or_default(),
            },
            outputs: cloned_config.outputs.unwrap_or_default(),
            output_paths: HashSet::new(),
            target: target.clone(),
            type_of: cloned_config.type_of,
        };

        debug!(
            target: &task.log_target,
            "Creating task {} with command {}",
            color::target(&target),
            color::shell(&task.command)
        );

        task
    }

    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> Result<glob::GlobSet, ProjectError> {
        Ok(glob::GlobSet::new(&self.input_globs)?)
    }

    /// Expand the args list to resolve tokens, relative to the project root.
    pub fn expand_args(&mut self, token_resolver: TokenResolver) -> Result<(), ProjectError> {
        if self.args.is_empty() {
            return Ok(());
        }

        let mut args: Vec<String> = vec![];
        let run_in_project = !self.options.run_from_workspace_root;

        // We cant use `TokenResolver.resolve` as args are a mix of strings,
        // strings with tokens, and file paths when tokens are resolved.
        for arg in &self.args {
            if token_resolver.has_token_func(arg) {
                for resolved_arg in token_resolver.resolve_func(arg, Some(self))? {
                    // When running within a project:
                    //  - Project paths are relative and start with "./"
                    //  - Workspace paths are absolute
                    // When running from the workspace:
                    //  - All paths are absolute
                    if run_in_project && resolved_arg.starts_with(token_resolver.data.project_root)
                    {
                        args.push(format!(
                            ".{}{}",
                            std::path::MAIN_SEPARATOR,
                            resolved_arg
                                .strip_prefix(token_resolver.data.project_root)
                                .unwrap()
                                .to_string_lossy()
                        ));
                    } else {
                        args.push(String::from(resolved_arg.to_string_lossy()));
                    }
                }
            } else if token_resolver.has_token_var(arg) {
                args.push(token_resolver.resolve_var(arg, self)?);
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
        depends_on: &[String],
    ) -> Result<(), ProjectError> {
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
                TargetProject::Deps => {
                    for project_id in depends_on {
                        push_dep(Target::format(project_id, &target.task_id)?);
                    }
                }
                // ~:task
                TargetProject::Own => {
                    push_dep(Target::format(owner_id, &target.task_id)?);
                }
                // project:task
                TargetProject::Id(_) => {
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

    /// Expand the inputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_inputs(&mut self, token_resolver: TokenResolver) -> Result<(), ProjectError> {
        if self.inputs.is_empty() {
            return Ok(());
        }

        for input in &token_resolver.resolve(&self.inputs, None)? {
            // We cant canonicalize here as these inputs may not exist!
            if glob::is_path_glob(input) {
                self.input_globs.push(glob::normalize(input)?);
            } else {
                self.input_paths.insert(path::normalize(input));
            }
        }

        Ok(())
    }

    /// Expand the outputs list to a set of absolute file paths, while resolving tokens.
    pub fn expand_outputs(&mut self, token_resolver: TokenResolver) -> Result<(), ProjectError> {
        if self.outputs.is_empty() {
            return Ok(());
        }

        for output in &token_resolver.resolve(&self.outputs, None)? {
            if glob::is_path_glob(output) {
                return Err(ProjectError::NoOutputGlob(
                    output.to_owned(),
                    self.target.clone(),
                ));
            } else {
                self.output_paths.insert(path::normalize(output));
            }
        }

        Ok(())
    }

    /// Return true if this task is affected, based on touched files.
    /// Will attempt to find any file that matches our list of inputs.
    pub fn is_affected(&self, touched_files: &TouchedFilePaths) -> Result<bool, ProjectError> {
        trace!(
            target: &self.log_target,
            "Checking if affected using input files: {}",
            map_list(&Vec::from_iter(self.input_paths.iter()), |p| color::path(p))
        );

        let has_globs = !self.input_globs.is_empty();
        let globset = self.create_globset()?;

        if has_globs {
            trace!(
                target: &self.log_target,
                "Checking if affected using input globs: {}",
                map_list(&self.input_globs, |f| color::file(f))
            );
        }

        for file in touched_files {
            let mut affected = self.input_paths.contains(file);

            if !affected && has_globs {
                affected = globset.matches(file)?;
            }

            trace!(
                target: &self.log_target,
                "Is affected by {} = {}",
                color::path(file),
                if affected {
                    color::success("true")
                } else {
                    color::failure("false")
                },
            );

            if affected {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn merge(&mut self, config: &TaskConfig) {
        // Merge options first incase the merge strategy has changed
        self.options.merge(&config.options);
        self.type_of = config.type_of.clone();

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

#[cfg(test)]
mod tests {
    use crate::test::create_expanded_task;
    use moon_config::TaskConfig;
    use moon_utils::string_vec;
    use moon_utils::test::get_fixtures_dir;
    use std::collections::HashSet;

    #[test]
    #[should_panic(expected = "NoOutputGlob")]
    fn errors_for_output_glob() {
        let workspace_root = get_fixtures_dir("projects");
        let project_root = workspace_root.join("basic");

        create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                outputs: Some(string_vec!["some/**/glob"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();
    }

    mod is_affected {
        use super::*;

        #[test]
        fn returns_true_if_matches_file() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(project_root.join("file.ts"));

            assert!(task.is_affected(&set).unwrap());
        }

        #[test]
        fn returns_true_if_matches_glob() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["file.*"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(project_root.join("file.ts"));

            assert!(task.is_affected(&set).unwrap());
        }

        #[test]
        fn returns_true_when_referencing_root_files() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["/package.json"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(workspace_root.join("package.json"));

            assert!(task.is_affected(&set).unwrap());
        }

        #[test]
        fn returns_false_if_outside_project() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["file.ts"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(workspace_root.join("base/other/outside.ts"));

            assert!(!task.is_affected(&set).unwrap());
        }

        #[test]
        fn returns_false_if_no_match() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let task = create_expanded_task(
                &workspace_root,
                &project_root,
                Some(TaskConfig {
                    inputs: Some(string_vec!["file.ts", "src/*"]),
                    ..TaskConfig::default()
                }),
            )
            .unwrap();

            let mut set = HashSet::new();
            set.insert(project_root.join("another.rs"));

            assert!(!task.is_affected(&set).unwrap());
        }
    }
}
