#![allow(dead_code)]

use crate::tasks_builder_error::TasksBuilderError;
use moon_common::{color, supports_pkl_configs, Id};
use moon_config::{
    is_glob_like, InheritedTasksConfig, InputPath, PlatformType, ProjectConfig,
    ProjectWorkspaceInheritedTasksConfig, TaskConfig, TaskDependency, TaskDependencyConfig,
    TaskMergeStrategy, TaskOptionsConfig, TaskOutputStyle, TaskType, ToolchainConfig,
};
use moon_target::Target;
use moon_task::{parse_task_args, Task, TaskOptions};
use moon_toolchain::detect::detect_task_platform;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::Path;
use tracing::{instrument, trace};

struct ConfigChain<'proj> {
    config: &'proj TaskConfig,
    inherited: bool,
}

#[instrument(skip(local_tasks, global_tasks))]
fn extract_config<'builder, 'proj>(
    task_id: &'builder Id,
    local_tasks: &'builder FxHashMap<&'proj Id, &'proj TaskConfig>,
    global_tasks: &'builder FxHashMap<&'proj Id, &'proj TaskConfig>,
) -> miette::Result<Vec<ConfigChain<'proj>>> {
    let mut stack = vec![];

    let mut extract = |tasks: &'builder FxHashMap<&'proj Id, &'proj TaskConfig>,
                       inherited: bool|
     -> miette::Result<()> {
        if let Some(config) = tasks.get(task_id) {
            stack.push(ConfigChain { config, inherited });

            if let Some(extend_task_id) = &config.extends {
                let extended_stack = extract_config(extend_task_id, local_tasks, global_tasks)?;

                if extended_stack.is_empty() {
                    return Err(TasksBuilderError::UnknownExtendsSource {
                        source_id: task_id.to_owned(),
                        target_id: extend_task_id.to_owned(),
                    }
                    .into());
                } else {
                    stack.extend(extended_stack);
                }
            }
        }

        Ok(())
    };

    extract(local_tasks, false)?;
    extract(global_tasks, true)?;

    Ok(stack)
}

pub struct TasksBuilderContext<'proj> {
    pub toolchain_config: &'proj ToolchainConfig,
    pub workspace_root: &'proj Path,
}

pub struct TasksBuilder<'proj> {
    context: TasksBuilderContext<'proj>,

    project_id: &'proj str,
    project_env: FxHashMap<&'proj str, &'proj str>,
    project_platform: &'proj PlatformType,
    project_source: &'proj str,

    // Global settings for tasks to inherit
    implicit_deps: Vec<&'proj TaskDependency>,
    implicit_inputs: Vec<&'proj InputPath>,

    // Tasks to merge and build
    task_ids: FxHashSet<&'proj Id>,
    global_tasks: FxHashMap<&'proj Id, &'proj TaskConfig>,
    global_task_options: Option<&'proj TaskOptionsConfig>,
    local_tasks: FxHashMap<&'proj Id, &'proj TaskConfig>,
    filters: Option<&'proj ProjectWorkspaceInheritedTasksConfig>,
}

impl<'proj> TasksBuilder<'proj> {
    pub fn new(
        project_id: &'proj str,
        project_source: &'proj str,
        project_platform: &'proj PlatformType,
        context: TasksBuilderContext<'proj>,
    ) -> Self {
        Self {
            context,
            project_id,
            project_env: FxHashMap::default(),
            project_platform,
            project_source,
            implicit_deps: vec![],
            implicit_inputs: vec![],
            task_ids: FxHashSet::default(),
            global_tasks: FxHashMap::default(),
            global_task_options: None,
            local_tasks: FxHashMap::default(),
            filters: None,
        }
    }

    #[instrument(skip_all)]
    pub fn inherit_global_tasks(
        &mut self,
        global_config: &'proj InheritedTasksConfig,
        global_filters: Option<&'proj ProjectWorkspaceInheritedTasksConfig>,
    ) -> &mut Self {
        let mut include_all = true;
        let mut include_set = FxHashSet::default();
        let mut exclude = vec![];
        let mut rename = FxHashMap::default();

        if let Some(filters) = global_filters {
            exclude.extend(&filters.exclude);
            rename.extend(&filters.rename);

            if let Some(include_config) = &filters.include {
                include_all = false;
                include_set.extend(include_config);
            }
        }

        trace!(
            id = self.project_id,
            tasks = ?global_config.tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
            "Filtering global tasks",
        );

        for (task_id, task_config) in &global_config.tasks {
            let target = Target::new(self.project_id, task_id).unwrap();

            // None = Include all
            // [] = Include none
            // ["a"] = Include "a"
            if !include_all {
                if include_set.is_empty() {
                    trace!(
                        target = target.as_str(),
                        "Not inheriting any global tasks, empty include filter",
                    );

                    break;
                } else if !include_set.contains(task_id) {
                    trace!(
                        target = target.as_str(),
                        "Not inheriting global task {}, not included",
                        color::id(task_id)
                    );

                    continue;
                }
            }

            // None, [] = Exclude none
            // ["a"] = Exclude "a"
            if !exclude.is_empty() && exclude.contains(&task_id) {
                trace!(
                    target = target.as_str(),
                    "Not inheriting global task {}, excluded",
                    color::id(task_id)
                );

                continue;
            }

            let task_key = if let Some(renamed_task_id) = rename.get(task_id) {
                trace!(
                    target = target.as_str(),
                    "Inheriting global task {} and renaming to {}",
                    color::id(task_id),
                    color::id(renamed_task_id)
                );

                renamed_task_id
            } else {
                trace!(
                    target = target.as_str(),
                    "Inheriting global task {}",
                    color::id(task_id),
                );

                task_id
            };

            self.global_tasks.insert(task_key, task_config);
            self.task_ids.insert(task_key);
        }

        self.filters = global_filters;
        self.global_task_options = global_config.task_options.as_ref();
        self.implicit_deps.extend(&global_config.implicit_deps);
        self.implicit_inputs.extend(&global_config.implicit_inputs);
        self
    }

    #[instrument(skip_all)]
    pub fn load_local_tasks(&mut self, local_config: &'proj ProjectConfig) -> &mut Self {
        for (key, value) in &local_config.env {
            self.project_env.insert(key, value);
        }

        trace!(
            id = self.project_id,
            tasks = ?local_config.tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
            "Loading local tasks",
        );

        self.local_tasks.extend(&local_config.tasks);

        for id in local_config.tasks.keys() {
            self.task_ids.insert(id);
        }

        self
    }

    #[instrument(name = "build_tasks", skip_all)]
    pub async fn build(self) -> miette::Result<BTreeMap<Id, Task>> {
        let mut tasks = BTreeMap::new();

        for id in &self.task_ids {
            tasks.insert((*id).to_owned(), self.build_task(id).await?);
        }

        Ok(tasks)
    }

    #[instrument(skip(self))]
    async fn build_task(&self, id: &Id) -> miette::Result<Task> {
        let target = Target::new(self.project_id, id)?;

        trace!(
            target = target.as_str(),
            "Building task {}",
            color::id(id.as_str())
        );

        let mut task = Task::default();
        let chain = self.get_config_inherit_chain(id)?;

        // Determine command and args before building options and the task,
        // as we need to figure out if we're running in local mode or not.
        let mut is_local = id == "dev" || id == "serve" || id == "start";
        let mut args_sets = vec![];

        for link in &chain {
            if let Some(local) = link.config.local {
                is_local = local;
            }

            if let Some((command, base_args)) = self.get_command_and_args(link.config)? {
                if let Some(command) = command {
                    task.command = command;
                }

                // Add to task later after we have a merge strategy
                args_sets.push(base_args);
            };
        }

        if is_local {
            trace!(target = target.as_str(), "Marking task as local");
        }

        task.options = self.build_task_options(id, is_local)?;
        task.metadata.local_only = is_local;
        task.metadata.root_level = self.project_source == ".";

        // Aggregate all values that are inherited from the global task configs,
        // and should always be included in the task, regardless of merge strategy.
        let global_deps = self.build_global_deps(&target)?;
        let mut global_inputs = self.build_global_inputs(&target, &task.options)?;

        // Aggregate all values that that are inherited from the project,
        // and should be set on the task first, so that merge strategies can be applied.
        for args in args_sets {
            task.args = self.merge_vec(task.args, args, task.options.merge_args, false);
        }

        task.env = self.build_env(&target)?;

        // Finally build the task itself, while applying our complex merge logic!
        let mut configured_inputs = 0;
        let mut has_configured_inputs = false;

        for link in &chain {
            let config = link.config;
            let deps = config
                .deps
                .iter()
                .cloned()
                .map(|d| d.into_config())
                .collect::<Vec<_>>();

            if config.script.is_some() {
                task.script = config.script.clone();
            }

            task.deps = self.merge_vec(
                task.deps,
                if link.inherited {
                    self.apply_filters_to_deps(deps)
                } else {
                    deps
                },
                task.options.merge_deps,
                true,
            );

            task.env = self.merge_map(task.env, config.env.to_owned(), task.options.merge_env);

            // Inherit global inputs as normal inputs, but do not consider them a configured input
            if !config.global_inputs.is_empty() {
                global_inputs.extend(config.global_inputs.to_owned());
            }

            // Inherit local inputs, which are user configured, and keep track of the total
            if let Some(inputs) = &config.inputs {
                has_configured_inputs = true;

                if inputs.is_empty()
                    && matches!(task.options.merge_inputs, TaskMergeStrategy::Replace)
                {
                    configured_inputs = 0;
                } else {
                    configured_inputs += inputs.len();
                }

                task.inputs = self.merge_vec(
                    task.inputs,
                    inputs.to_owned(),
                    task.options.merge_inputs,
                    true,
                );
            }

            if let Some(outputs) = &config.outputs {
                task.outputs = self.merge_vec(
                    task.outputs,
                    outputs.to_owned(),
                    task.options.merge_outputs,
                    true,
                );
            }

            if !config.platform.is_unknown() {
                task.platform = config.platform;
            }

            if config.description.is_some() {
                task.description = config.description.clone();
            }
        }

        // Inputs are tricky, as they come from many sources. We need to ensure that user configured
        // inputs are handled explicitly, while globally inherited sources are handled implicitly.
        if configured_inputs == 0 {
            if has_configured_inputs {
                trace!(
                    target = target.as_str(),
                    "Task has explicitly disabled inputs",
                );

                task.metadata.empty_inputs = true;
            } else if task.metadata.root_level {
                trace!(
                    target = target.as_str(),
                    "Task is in a root-level project, defaulting to no inputs",
                );

                task.metadata.empty_inputs = true;
            } else {
                trace!(
                    target = target.as_str(),
                    "No inputs configured, defaulting to {} (from project)",
                    color::file("**/*"),
                );

                task.inputs.push(InputPath::ProjectGlob("**/*".into()));
            }
        }

        // If a script, wipe out inherited arguments, and extract the first command
        if let Some(script) = &task.script {
            task.args.clear();
            task.platform = PlatformType::System;

            if let Some(i) = script.find(' ') {
                task.command = script[0..i].to_owned();
            } else {
                task.command = script.to_owned();
            }
        }

        // And lastly, before we return the task and options, we should finalize
        // all necessary fields and populate/calculate with values.
        if task.command.is_empty() {
            task.command = "noop".into();
        }

        if !global_deps.is_empty() {
            task.deps = self.merge_vec(task.deps, global_deps, TaskMergeStrategy::Append, true);
        }

        task.id = id.to_owned();

        if !global_inputs.is_empty() {
            task.inputs =
                self.merge_vec(task.inputs, global_inputs, TaskMergeStrategy::Append, true);
        }

        if task.platform.is_unknown() {
            let platform = detect_task_platform(
                &task.command,
                &self.context.toolchain_config.get_enabled_platforms(),
            );

            task.platform = if platform.is_unknown() {
                if self.project_platform.is_unknown() {
                    PlatformType::System
                } else {
                    self.project_platform.to_owned()
                }
            } else {
                platform
            };
        }

        task.type_of = if !task.outputs.is_empty() {
            TaskType::Build
        } else if is_local {
            TaskType::Run
        } else {
            TaskType::Test
        };

        if task.options.shell.is_none() {
            // Windows requires a shell for path resolution to work correctly
            if cfg!(windows) || task.platform.is_system() || task.script.is_some() {
                task.options.shell = Some(true);
            }

            // If an arg contains a glob, we must run in a shell for expansion to work
            if task.args.iter().any(|a| is_glob_like(a)) {
                trace!(
                    target = target.as_str(),
                    "Task has a glob-like argument, wrapping in a shell so glob expansion works",
                );

                task.options.shell = Some(true);
            }
        }

        task.target = target;

        Ok(task)
    }

    #[instrument(skip(self))]
    fn build_task_options(&self, id: &Id, is_local: bool) -> miette::Result<TaskOptions> {
        let mut options = TaskOptions {
            cache: !is_local,
            interactive: false,
            output_style: is_local.then_some(TaskOutputStyle::Stream),
            persistent: is_local,
            run_in_ci: !is_local,
            ..TaskOptions::default()
        };

        let mut chain = vec![];

        if let Some(default_options) = self.global_task_options {
            chain.push(default_options);
        }

        chain.extend(
            self.get_config_inherit_chain(id)?
                .iter()
                .map(|link| &link.config.options)
                .collect::<Vec<_>>(),
        );

        for config in chain {
            if let Some(affected_files) = &config.affected_files {
                options.affected_files = Some(affected_files.to_owned());
            }

            if let Some(affected_pass_inputs) = &config.affected_pass_inputs {
                options.affected_pass_inputs = *affected_pass_inputs;
            }

            if let Some(allow_failure) = &config.allow_failure {
                options.allow_failure = *allow_failure;
            }

            if let Some(cache) = &config.cache {
                options.cache = *cache;
            }

            if let Some(env_file) = &config.env_file {
                options.env_files = env_file.to_input_paths();
            }

            if let Some(internal) = &config.internal {
                options.internal = *internal;
            }

            if let Some(interactive) = &config.interactive {
                options.interactive = *interactive;
            }

            if let Some(merge_args) = &config.merge_args {
                options.merge_args = *merge_args;
            }

            if let Some(merge_deps) = &config.merge_deps {
                options.merge_deps = *merge_deps;
            }

            if let Some(merge_env) = &config.merge_env {
                options.merge_env = *merge_env;
            }

            if let Some(merge_inputs) = &config.merge_inputs {
                options.merge_inputs = *merge_inputs;
            }

            if let Some(merge_outputs) = &config.merge_outputs {
                options.merge_outputs = *merge_outputs;
            }

            if let Some(mutex) = &config.mutex {
                options.mutex = Some(mutex.clone());
            }

            if let Some(output_style) = &config.output_style {
                options.output_style = Some(*output_style);
            }

            if let Some(persistent) = &config.persistent {
                options.persistent = *persistent;
            }

            if let Some(retry_count) = &config.retry_count {
                options.retry_count = *retry_count;
            }

            if let Some(run_deps_in_parallel) = &config.run_deps_in_parallel {
                options.run_deps_in_parallel = *run_deps_in_parallel;
            }

            if let Some(run_in_ci) = &config.run_in_ci {
                options.run_in_ci = *run_in_ci;
            }

            if let Some(run_from_workspace_root) = &config.run_from_workspace_root {
                options.run_from_workspace_root = *run_from_workspace_root;
            }

            if let Some(shell) = &config.shell {
                options.shell = Some(*shell);
            }

            if let Some(timeout) = &config.timeout {
                options.timeout = Some(*timeout);
            }

            if let Some(unix_shell) = &config.unix_shell {
                options.unix_shell = Some(*unix_shell);
            }

            if let Some(windows_shell) = &config.windows_shell {
                options.windows_shell = Some(*windows_shell);
            }
        }

        if options.interactive {
            options.cache = false;
            options.output_style = Some(TaskOutputStyle::Stream);
            options.persistent = false;
            options.run_in_ci = false;
        }

        Ok(options)
    }

    fn build_global_deps(&self, target: &Target) -> miette::Result<Vec<TaskDependencyConfig>> {
        let global_deps = self
            .implicit_deps
            .iter()
            .map(|d| (*d).to_owned().into_config())
            .collect::<Vec<_>>();

        if !global_deps.is_empty() {
            trace!(
                target = target.as_str(),
                deps = ?global_deps.iter().map(|d| d.target.as_str()).collect::<Vec<_>>(),
                "Inheriting global implicit deps",
            );
        }

        Ok(global_deps)
    }

    fn build_global_inputs(
        &self,
        target: &Target,
        options: &TaskOptions,
    ) -> miette::Result<Vec<InputPath>> {
        let mut global_inputs = self
            .implicit_inputs
            .iter()
            .map(|d| (*d).to_owned())
            .collect::<Vec<_>>();

        global_inputs.push(InputPath::WorkspaceGlob(".moon/*.yml".into()));

        if supports_pkl_configs() {
            global_inputs.push(InputPath::WorkspaceGlob(".moon/*.pkl".into()));
        }

        if let Some(env_files) = &options.env_files {
            global_inputs.extend(env_files.to_owned());
        }

        if !global_inputs.is_empty() {
            trace!(
                target = target.as_str(),
                inputs = ?global_inputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
                "Inheriting global implicit inputs",
            );
        }

        Ok(global_inputs)
    }

    fn build_env(&self, target: &Target) -> miette::Result<FxHashMap<String, String>> {
        let env = self
            .project_env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<FxHashMap<_, _>>();

        if !env.is_empty() {
            trace!(
                target = target.as_str(),
                env_vars = ?self.project_env,
                "Inheriting project env vars",
            );
        }

        Ok(env)
    }

    fn get_command_and_args(
        &self,
        config: &TaskConfig,
    ) -> miette::Result<Option<(Option<String>, Vec<String>)>> {
        if config.script.is_some() {
            return Ok(None);
        }

        let mut command = None;
        let mut args = vec![];
        let mut cmd_list = parse_task_args(&config.command)?;

        if !cmd_list.is_empty() {
            command = Some(cmd_list.remove(0));
            args.extend(cmd_list);
        }

        args.extend(parse_task_args(&config.args)?);

        Ok(Some((command, args)))
    }

    fn get_config_inherit_chain(&self, id: &Id) -> miette::Result<Vec<ConfigChain>> {
        let mut stack = extract_config(id, &self.local_tasks, &self.global_tasks)?;
        stack.reverse();

        Ok(stack)
    }

    fn apply_filters_to_deps(&self, deps: Vec<TaskDependencyConfig>) -> Vec<TaskDependencyConfig> {
        let Some(filters) = &self.filters else {
            return deps;
        };

        deps.into_iter()
            .filter(|dep| !filters.exclude.contains(&dep.target.task_id))
            .map(|mut dep| {
                if let Some(new_task_id) = filters.rename.get(&dep.target.task_id) {
                    dep.target.id = Target::format(&dep.target.scope, new_task_id);
                    dep.target.task_id = new_task_id.to_owned();
                }

                dep
            })
            .collect()
    }

    fn merge_map<K, V>(
        &self,
        base: FxHashMap<K, V>,
        next: FxHashMap<K, V>,
        strategy: TaskMergeStrategy,
    ) -> FxHashMap<K, V>
    where
        K: Eq + Hash,
    {
        match strategy {
            TaskMergeStrategy::Append => {
                if next.is_empty() {
                    return base;
                }

                let mut map = FxHashMap::default();
                map.extend(base);
                map.extend(next);
                map
            }
            TaskMergeStrategy::Prepend => {
                if next.is_empty() {
                    return base;
                }

                let mut map = FxHashMap::default();
                map.extend(next);
                map.extend(base);
                map
            }
            TaskMergeStrategy::Replace => next,
        }
    }

    fn merge_vec<T: Eq>(
        &self,
        base: Vec<T>,
        next: Vec<T>,
        strategy: TaskMergeStrategy,
        dedupe: bool,
    ) -> Vec<T> {
        let mut list: Vec<T> = vec![];

        // Dedupe while merging vectors. We can't use a set here because
        // we need to preserve the insertion order. Revisit if this is costly!
        let mut append = |items: Vec<T>, force: bool| {
            for item in items {
                #[allow(clippy::nonminimal_bool)]
                if force || !dedupe || (dedupe && !list.contains(&item)) {
                    list.push(item);
                }
            }
        };

        match strategy {
            TaskMergeStrategy::Append => {
                if next.is_empty() {
                    return base;
                }

                append(base, true);
                append(next, false);
            }
            TaskMergeStrategy::Prepend => {
                if next.is_empty() {
                    return base;
                }

                append(next, true);
                append(base, false);
            }
            TaskMergeStrategy::Replace => {
                list.extend(next);
            }
        }

        list
    }
}
