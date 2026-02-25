#![allow(dead_code)]

use crate::tasks_builder_error::TasksBuilderError;
use indexmap::{IndexMap, IndexSet};
use moon_common::{
    Id, color,
    path::{WorkspaceRelativePath, encode_component, is_root_level_source},
};
use moon_config::{
    ConfigLoader, EnvMap, InheritedTasksConfig, Input, ProjectConfig, ProjectDependencyConfig,
    ProjectInput, ProjectWorkspaceInheritedTasksConfig, TaskArgs, TaskConfig, TaskDependency,
    TaskDependencyConfig, TaskMergeStrategy, TaskOptionAffectedFilesEntry, TaskOptionCache,
    TaskOptionRunInCI, TaskOptionsConfig, TaskOutputStyle, TaskPreset, TaskPriority, TaskType,
    ToolchainsConfig, is_glob_like,
};
use moon_env_var::contains_env_var;
use moon_target::Target;
use moon_task::{
    Task, TaskArg, TaskOptionAffectedFiles, TaskOptionEnvFile, TaskOptions, TaskState,
};
use moon_toolchain::filter_and_resolve_toolchain_ids;
use moon_toolchain_plugin::{ToolchainRegistry, api::DefineRequirementsInput};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::Path;
use std::sync::Arc;
use tracing::{instrument, trace};

#[derive(Debug, Default)]
struct CommandLineParseResult {
    pub command: Option<TaskArg>,
    pub args: Option<Vec<TaskArg>>,
    pub env: Option<EnvMap>,
    pub requires_shell: bool,
}

struct ConfigChain<'proj> {
    config: &'proj TaskConfig,
    inherited: bool,
}

#[instrument(skip(local_tasks, global_tasks))]
fn extract_config<'builder, 'proj>(
    task_id: &'builder Id,
    local_tasks: &'builder FxHashMap<&'proj Id, &'proj TaskConfig>,
    global_tasks: &'builder FxHashMap<&'proj Id, Vec<&'proj TaskConfig>>,
) -> miette::Result<Vec<ConfigChain<'proj>>> {
    let mut stack = vec![];

    let mut extract = |config: &'proj TaskConfig, inherited: bool| -> miette::Result<()> {
        if let Some(extend_task_id) = &config.extends {
            let extended_stack = extract_config(extend_task_id, local_tasks, global_tasks)?;

            if extended_stack.is_empty() {
                return Err(TasksBuilderError::UnknownExtendsSource {
                    source_id: task_id.to_string(),
                    target_id: extend_task_id.to_string(),
                }
                .into());
            } else {
                stack.extend(extended_stack);
            }
        }

        stack.push(ConfigChain { config, inherited });

        Ok(())
    };

    if let Some(configs) = global_tasks.get(task_id) {
        for config in configs {
            extract(config, true)?;
        }
    }

    if let Some(config) = local_tasks.get(task_id) {
        extract(config, false)?;
    }

    Ok(stack)
}

#[derive(Debug)]
pub struct TasksBuilderContext<'proj> {
    pub config_loader: &'proj ConfigLoader,
    pub enabled_toolchains: &'proj [Id],
    pub monorepo: bool,
    pub toolchains_config: &'proj ToolchainsConfig,
    pub toolchain_registry: Arc<ToolchainRegistry>,
    pub workspace_root: &'proj Path,
}

#[derive(Debug)]
pub struct TasksBuilder<'proj> {
    context: TasksBuilderContext<'proj>,

    project_id: &'proj Id,
    project_dependencies: &'proj [ProjectDependencyConfig],
    project_env: FxHashMap<&'proj str, Option<&'proj str>>,
    project_source: &'proj WorkspaceRelativePath,
    project_toolchains: &'proj [Id],

    // Global settings for tasks to inherit
    implicit_deps: Vec<&'proj TaskDependency>,
    implicit_inputs: Vec<&'proj Input>,

    // Tasks to merge and build
    task_ids: FxHashSet<&'proj Id>,
    global_tasks: FxHashMap<&'proj Id, Vec<&'proj TaskConfig>>,
    global_task_options: Vec<&'proj TaskOptionsConfig>,
    local_tasks: FxHashMap<&'proj Id, &'proj TaskConfig>,
    filters: Option<&'proj ProjectWorkspaceInheritedTasksConfig>,
}

impl<'proj> TasksBuilder<'proj> {
    pub fn new(
        project_id: &'proj Id,
        project_dependencies: &'proj [ProjectDependencyConfig],
        project_source: &'proj WorkspaceRelativePath,
        project_toolchains: &'proj [Id],
        context: TasksBuilderContext<'proj>,
    ) -> Self {
        Self {
            context,
            project_id,
            project_dependencies,
            project_env: FxHashMap::default(),
            project_source,
            project_toolchains,
            implicit_deps: vec![],
            implicit_inputs: vec![],
            task_ids: FxHashSet::default(),
            global_tasks: FxHashMap::default(),
            global_task_options: vec![],
            local_tasks: FxHashMap::default(),
            filters: None,
        }
    }

    #[instrument(skip_all)]
    pub fn inherit_global_tasks(
        &mut self,
        global_configs: &'proj IndexMap<String, InheritedTasksConfig>,
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

        if include_set.is_empty() {
            trace!(
                project_id = self.project_id.as_str(),
                "Not inheriting any global tasks, empty include filter",
            );
        } else {
            trace!(
                project_id = self.project_id.as_str(),
                "Inheriting and filtering global tasks",
            );
        }

        for global_config in global_configs.values() {
            for (task_id, task_config) in &global_config.tasks {
                let target = Target::new(self.project_id, task_id).unwrap();

                // None = Include all
                // [] = Include none
                // ["a"] = Include "a"
                if !include_all {
                    if include_set.is_empty() {
                        break;
                    } else if !include_set.contains(task_id) {
                        trace!(
                            task_target = target.as_str(),
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
                        task_target = target.as_str(),
                        "Not inheriting global task {}, excluded",
                        color::id(task_id)
                    );

                    continue;
                }

                let task_key = if let Some(renamed_task_id) = rename.get(task_id) {
                    trace!(
                        task_target = target.as_str(),
                        "Inheriting global task {} and renaming to {}",
                        color::id(task_id),
                        color::id(renamed_task_id)
                    );

                    renamed_task_id
                } else {
                    trace!(
                        task_target = target.as_str(),
                        "Inheriting global task {}",
                        color::id(task_id),
                    );

                    task_id
                };

                self.global_tasks
                    .entry(task_key)
                    .or_default()
                    .push(task_config);
                self.task_ids.insert(task_key);
            }

            if let Some(options) = &global_config.task_options {
                self.global_task_options.push(options);
            }

            self.implicit_deps.extend(&global_config.implicit_deps);
            self.implicit_inputs.extend(&global_config.implicit_inputs);
        }

        self.filters = global_filters;
        self
    }

    #[instrument(skip_all)]
    pub fn load_local_tasks(&mut self, local_config: &'proj ProjectConfig) -> &mut Self {
        for (key, value) in &local_config.env {
            self.project_env.insert(key, value.as_deref());
        }

        trace!(
            project_id = self.project_id.as_str(),
            task_ids = ?local_config.tasks.keys().map(|k| k.as_str()).collect::<Vec<_>>(),
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
            task_target = target.as_str(),
            "Building task {}",
            color::id(id.as_str())
        );

        let mut task = Task {
            // Reset toolchains so that we don't inherit system by default
            toolchains: vec![],
            ..Default::default()
        };
        let mut state = TaskState::default();

        // Determine command and args before building options and the task,
        // as we need to figure out if we're running in local mode or not.
        let mut preset = None;
        let mut args_sets = vec![];
        let mut env_sets = vec![];
        let mut requires_shell = false;

        if id == "dev" || id == "serve" || id == "start" {
            preset = Some(TaskPreset::Server);
        }

        let chain = self.get_config_inherit_chain(id)?;

        for link in &chain {
            if let Some(pre) = link.config.preset {
                preset = Some(pre);
            }

            if let Some(command_line) = self.get_command_line(&target, link.config)? {
                if let Some(command) = command_line.command {
                    task.command = command;
                }

                if let Some(args) = command_line.args {
                    args_sets.push(args);
                }

                if let Some(env) = command_line.env {
                    env_sets.push(env);
                }

                if command_line.requires_shell {
                    requires_shell = true;
                }
            }
        }

        task.preset = preset;
        task.options = self.build_task_options(id, preset, &mut state)?;
        task.env = self.inherit_project_env(&target)?;
        state.root_level = is_root_level_source(self.project_source);

        // Aggregate all values that are inherited from the global task configs,
        // and should always be included in the task, regardless of merge strategy.
        let global_deps = self.inherit_global_deps(&target)?;
        let mut global_inputs = self.inherit_global_inputs(&target, &task.options)?;

        // Aggregate all values that that are inherited from the project,
        // and should be set on the task first, so that merge strategies can be applied.
        for (index, args) in args_sets.into_iter().enumerate() {
            task.args = self.merge_vec(task.args, args, task.options.merge_args, index, false);
        }

        for (index, env) in env_sets.into_iter().enumerate() {
            task.env = self.merge_index_map(task.env, env, task.options.merge_env, index);
        }

        // Finally build the task itself, while applying our complex merge logic!
        let mut configured_inputs = 0;
        let mut has_configured_inputs = false;
        let mut has_set_type = false;

        for (index, link) in chain.iter().enumerate() {
            let config = link.config;

            if config.script.is_some() {
                task.script = config.script.clone();
            }

            if let Some(deps) = &config.deps {
                let deps = deps
                    .iter()
                    .cloned()
                    .map(|dep| dep.into_config())
                    .collect::<Vec<_>>();

                task.deps = self.merge_vec(
                    task.deps,
                    if link.inherited {
                        self.apply_filters_to_deps(deps)
                    } else {
                        deps
                    },
                    task.options.merge_deps,
                    index,
                    true,
                );
            }

            if let Some(env) = &config.env {
                task.env =
                    self.merge_index_map(task.env, env.to_owned(), task.options.merge_env, index);
            }

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
                    index,
                    true,
                );
            }

            if let Some(outputs) = &config.outputs {
                task.outputs = self.merge_vec(
                    task.outputs,
                    outputs.to_owned(),
                    task.options.merge_outputs,
                    index,
                    true,
                );
            }

            if let Some(toolchains) = &config.toolchains {
                task.toolchains = self.merge_vec(
                    task.toolchains,
                    toolchains.to_owned_list(),
                    task.options.merge_toolchains,
                    index,
                    true,
                );
            }

            if config.description.is_some() {
                task.description = config.description.clone();
            }

            if let Some(ty) = config.type_of {
                task.type_of = ty;
                has_set_type = true;
            }
        }

        // Inputs are tricky, as they come from many sources. We need to ensure that user configured
        // inputs are handled explicitly, while globally inherited sources are handled implicitly.
        if configured_inputs == 0 {
            if has_configured_inputs {
                trace!(
                    task_target = target.as_str(),
                    "Task has explicitly disabled inputs",
                );

                state.empty_inputs = true;
            } else if self.context.monorepo && state.root_level {
                trace!(
                    task_target = target.as_str(),
                    "Task is a root-level project in a monorepo, defaulting to no inputs",
                );

                state.empty_inputs = true;
            } else {
                trace!(
                    task_target = target.as_str(),
                    "No inputs configured, defaulting to {} (from project)",
                    color::file("**/*"),
                );

                task.inputs.push(Input::parse("**/*").unwrap());
                state.default_inputs = true;
            }
        } else if configured_inputs == 1
            && task
                .inputs
                .first()
                .is_some_and(|first| first.as_str() == "**/*")
        {
            state.default_inputs = true;
        }

        // If a script, wipe out inherited arguments, and extract the first command
        if let Some(script) = &task.script {
            task.args.clear();

            if let Some(i) = script.find(' ') {
                task.command = TaskArg::new(&script[0..i]);
            } else {
                task.command = TaskArg::new(script);
            }

            trace!(
                task_target = target.as_str(),
                "Task has defined a shell script, wrapping in a shell as its required",
            );
        }

        // And lastly, before we return the task and options, we should finalize
        // all necessary fields and populate/calculate with values.

        if task.command.is_empty() {
            task.command = TaskArg::new_unquoted("noop");
        }

        if !global_deps.is_empty() {
            task.deps = self.merge_vec(
                task.deps,
                global_deps,
                TaskMergeStrategy::Append,
                1000,
                true,
            );
        }

        if !global_inputs.is_empty() {
            task.inputs = self.merge_vec(
                task.inputs,
                global_inputs,
                TaskMergeStrategy::Append,
                1000,
                true,
            );
        }

        if !has_set_type {
            task.type_of = if !task.outputs.is_empty() {
                TaskType::Build
            } else if let Some(set) = preset {
                set.get_type()
            } else if task.options.persistent {
                TaskType::Run
            } else {
                TaskType::Test
            };
        }

        if !state.set_run_in_ci {
            task.options.run_in_ci = TaskOptionRunInCI::Enabled(matches!(
                task.type_of,
                TaskType::Build | TaskType::Test
            ));
        }

        if state.shell_disabled {
            requires_shell = false;
        } else {
            // If an arg contains a glob, we must run in a shell for expansion to work
            if task.args.iter().any(|arg| is_glob_like(arg)) {
                trace!(
                    task_target = target.as_str(),
                    "Task has a glob-like argument, wrapping in a shell so glob expansion works",
                );

                requires_shell = true;
            }

            // If an arg contains an env var, we must run in a shell for substitution to work
            if contains_env_var(&task.command) || task.args.iter().any(contains_env_var) {
                trace!(
                    task_target = target.as_str(),
                    "Task references an environment variable, wrapping in a shell so substitution works",
                );

                requires_shell = true;
            }
        }

        if requires_shell || task.script.is_some() {
            task.options.shell = Some(true);
        }

        if let Some(os_list) = &task.options.os {
            let for_current_system = os_list.iter().any(|os| os.is_current_system());

            if !for_current_system {
                trace!(
                    task_target = target.as_str(),
                    os_list = ?os_list.iter().map(|os| os.to_string()).collect::<Vec<_>>(),
                    "Task has been marked for another operating system, disabling command/script",
                );

                task.command = TaskArg::new_unquoted("noop");
                task.args.clear();
                task.script = None;
            }
        }

        task.id = id.to_owned();
        task.target = target;
        task.state = state;

        self.resolve_task_inputs(&mut task)?;
        self.resolve_task_toolchains(&mut task).await?;

        Ok(task)
    }

    fn build_task_options(
        &self,
        id: &Id,
        preset: Option<TaskPreset>,
        state: &mut TaskState,
    ) -> miette::Result<TaskOptions> {
        let mut options = self.get_task_options_from_preset(preset, state);
        let mut chain = self.global_task_options.clone();

        chain.extend(
            self.get_config_inherit_chain(id)?
                .iter()
                .map(|link| &link.config.options)
                .collect::<Vec<_>>(),
        );

        for config in chain {
            if let Some(affected_files) = &config.affected_files {
                let mut option = TaskOptionAffectedFiles::default();

                match affected_files {
                    TaskOptionAffectedFilesEntry::Pattern(pat) => {
                        option.pass = pat.to_owned();
                    }
                    TaskOptionAffectedFilesEntry::Object(opt) => {
                        option.pass = opt.pass.clone();
                        option.pass_inputs_when_no_match =
                            opt.pass_inputs_when_no_match.unwrap_or_default();
                    }
                };

                options.affected_files = Some(option);
            }

            if let Some(allow_failure) = &config.allow_failure {
                options.allow_failure = *allow_failure;
            }

            if let Some(cache) = &config.cache {
                options.cache = cache.to_owned();
            }

            if let Some(cache_key) = &config.cache_key {
                options.cache_key = Some(cache_key.to_owned());
            }

            if let Some(cache_lifetime) = &config.cache_lifetime {
                options.cache_lifetime = Some(cache_lifetime.to_owned());
            }

            if let Some(env_file) = &config.env_file {
                options.env_files = self.resolve_env_files(id, env_file)?;
            }

            if let Some(infer_inputs) = &config.infer_inputs {
                options.infer_inputs = *infer_inputs;
            }

            if let Some(internal) = &config.internal {
                options.internal = *internal;
            }

            if let Some(interactive) = &config.interactive {
                options.interactive = *interactive;
            }

            if let Some(merge) = &config.merge {
                options.merge_args = *merge;
                options.merge_deps = *merge;
                options.merge_env = *merge;
                options.merge_inputs = *merge;
                options.merge_outputs = *merge;
                options.merge_toolchains = *merge;
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

            if let Some(merge_toolchains) = &config.merge_toolchains {
                options.merge_toolchains = *merge_toolchains;
            }

            if let Some(mutex) = &config.mutex {
                options.mutex = Some(mutex.to_owned());
            }

            if let Some(os) = &config.os {
                options.os = Some(os.to_owned_list());
            }

            if let Some(output_style) = &config.output_style {
                options.output_style = Some(*output_style);
            }

            if let Some(persistent) = &config.persistent {
                options.persistent = *persistent;
            }

            if let Some(priority) = &config.priority {
                options.priority = *priority;
            }

            if let Some(retry_count) = &config.retry_count {
                options.retry_count = *retry_count;
            }

            if let Some(run_deps_in_parallel) = &config.run_deps_in_parallel {
                options.run_deps_in_parallel = *run_deps_in_parallel;
            }

            if let Some(run_in_ci) = &config.run_in_ci {
                options.run_in_ci = run_in_ci.to_owned();
                state.set_run_in_ci = true;
            }

            if let Some(run_from_workspace_root) = &config.run_from_workspace_root {
                options.run_from_workspace_root = *run_from_workspace_root;
            }

            if let Some(shell) = &config.shell {
                options.shell = Some(*shell);
                state.shell_disabled = !shell;
            }

            if let Some(timeout) = &config.timeout {
                options.timeout = Some(*timeout);
            }

            if let Some(unix_shell) = &config.unix_shell {
                options.unix_shell = *unix_shell;
            }

            if let Some(windows_shell) = &config.windows_shell {
                options.windows_shell = *windows_shell;
            }
        }

        // Interactive has special handling
        if options.interactive {
            options.output_style = Some(TaskOutputStyle::Stream);

            if options.run_in_ci != TaskOptionRunInCI::Skip {
                options.run_in_ci = TaskOptionRunInCI::Enabled(false);
                state.set_run_in_ci = true;
            }
        }

        Ok(options)
    }

    fn resolve_env_files(
        &self,
        task_id: &Id,
        option: &TaskOptionEnvFile,
    ) -> miette::Result<Option<Vec<Input>>> {
        let mut list = vec![];

        match option {
            TaskOptionEnvFile::Enabled(true) => {
                let encoded_task_id = encode_component(task_id);

                for path in [
                    "/.env".to_owned(),
                    "/.env.local".to_owned(),
                    ".env".to_owned(),
                    ".env.local".to_owned(),
                    format!(".env.{encoded_task_id}"),
                    format!(".env.{encoded_task_id}.local"),
                ] {
                    list.push(Input::parse(&path)?);
                }
            }
            TaskOptionEnvFile::Enabled(false) => {}
            TaskOptionEnvFile::File(path) => {
                list.push(Input::parse(path.as_str())?);
            }
            TaskOptionEnvFile::Files(paths) => {
                for path in paths {
                    list.push(Input::parse(path.as_str())?);
                }
            }
        };

        Ok(if list.is_empty() { None } else { Some(list) })
    }

    fn resolve_task_inputs(&self, task: &mut Task) -> miette::Result<()> {
        let mut inputs = vec![];

        for input in std::mem::take(&mut task.inputs) {
            if let Input::Project(inner) = input {
                if inner.is_all_deps() {
                    for dep_config in self.project_dependencies {
                        inputs.push(Input::Project(ProjectInput {
                            project: dep_config.id.to_string(),
                            filter: inner.filter.clone(),
                            group: inner.group.clone(),
                        }));
                    }
                } else if self
                    .project_dependencies
                    .iter()
                    .any(|dep| dep.id == inner.project)
                {
                    inputs.push(Input::Project(inner));
                } else {
                    return Err(TasksBuilderError::UnknownProjectInput {
                        dep: inner.project,
                        task: task.target.clone(),
                    }
                    .into());
                }
            } else {
                inputs.push(input);
            }
        }

        task.inputs = inputs;

        Ok(())
    }

    async fn resolve_task_toolchains(&self, task: &mut Task) -> miette::Result<()> {
        let mut toolchains = IndexSet::<Id>::default();

        // Implicitly detected/inherited toolchains
        if task.toolchains.is_empty() {
            toolchains.extend(
                self.context
                    .toolchain_registry
                    .detect_task_usage(
                        self.context.enabled_toolchains.iter().collect(),
                        &task.command.value,
                    )
                    .await?,
            );
        }
        // Explicitly configured toolchains
        else {
            toolchains.extend(task.toolchains.clone());
        }

        // If none, then inherit the toolchains from the project
        if toolchains.is_empty() {
            toolchains.extend(self.project_toolchains.to_owned());
        }

        // Expand the toolchains list with required/dependency relationships
        let toolchains = self
            .context
            .toolchain_registry
            .expand_task_usage(toolchains.into_iter().collect(), |registry, toolchain| {
                DefineRequirementsInput {
                    context: registry.create_context(),
                    toolchain_config: registry.create_config(&toolchain.id),
                }
            })
            .await?;

        // Resolve them to valid identifiers
        task.toolchains =
            filter_and_resolve_toolchain_ids(self.context.enabled_toolchains, toolchains, true);

        Ok(())
    }

    fn inherit_global_deps(&self, target: &Target) -> miette::Result<Vec<TaskDependencyConfig>> {
        let global_deps = self
            .implicit_deps
            .iter()
            .map(|dep| (*dep).to_owned().into_config())
            .collect::<Vec<_>>();

        if !global_deps.is_empty() {
            trace!(
                task_target = target.as_str(),
                dep_targets = ?global_deps.iter().map(|d| d.target.as_str()).collect::<Vec<_>>(),
                "Inheriting global implicit deps",
            );
        }

        Ok(global_deps)
    }

    fn inherit_global_inputs(
        &self,
        target: &Target,
        options: &TaskOptions,
    ) -> miette::Result<Vec<Input>> {
        let mut global_inputs = self
            .implicit_inputs
            .iter()
            .map(|dep| (*dep).to_owned())
            .collect::<Vec<_>>();

        global_inputs.push(
            Input::parse(format!(
                "/.moon/*.{}",
                self.context.config_loader.get_ext_glob()
            ))
            .unwrap(),
        );

        if let Some(env_files) = &options.env_files {
            global_inputs.extend(env_files.to_owned());
        }

        if !global_inputs.is_empty() {
            trace!(
                task_target = target.as_str(),
                inputs = ?global_inputs.iter().map(|d| d.as_str()).collect::<Vec<_>>(),
                "Inheriting global implicit inputs",
            );
        }

        Ok(global_inputs)
    }

    fn inherit_project_env(&self, target: &Target) -> miette::Result<EnvMap> {
        let env = self
            .project_env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).map(|v| v.to_string())))
            .collect::<EnvMap>();

        if !env.is_empty() {
            trace!(
                task_target = target.as_str(),
                env_vars = ?self.project_env,
                "Inheriting project env vars",
            );
        }

        Ok(env)
    }

    fn parse_command_line(
        &self,
        target: &Target,
        args: &TaskArgs,
    ) -> miette::Result<CommandLineParseResult> {
        let mut res = CommandLineParseResult::default();

        match args {
            TaskArgs::Noop => Ok(res),
            TaskArgs::List(list) => {
                res.args = Some(list.iter().map(TaskArg::new).collect());

                Ok(res)
            }
            TaskArgs::String(cmd) => {
                use starbase_args::*;

                if cmd.is_empty() {
                    return Ok(res);
                }

                let mut args = vec![];
                let mut env = EnvMap::default();

                let mut handle_arg = |arg: &Argument, extract_env: bool| {
                    if let Argument::Value(value) = arg {
                        if !res.requires_shell
                            && matches!(
                                value,
                                Value::Expansion(_)
                                    | Value::Substitution(_)
                                    | Value::MurexBraceQuoted(_)
                                    | Value::NuRawQuoted(_)
                            )
                        {
                            res.requires_shell = true;
                        }

                        if value.is_quoted() {
                            args.push(TaskArg::new_quoted(
                                value.get_quoted_value(),
                                value.to_string(),
                            ));
                        } else {
                            args.push(TaskArg::new_unquoted(value.to_string()));
                        }
                    } else if let Argument::EnvVar(key, value, _) = arg {
                        if extract_env {
                            if !matches!(value, Value::Expansion(_) | Value::Substitution(_)) {
                                env.insert(key.to_owned(), Some(value.as_str().to_owned()));
                            }
                        } else {
                            args.push(TaskArg::new_unquoted(arg.to_string()));
                        }
                    } else {
                        args.push(TaskArg::new_unquoted(arg.to_string()));
                    }
                };

                let command_line =
                    parse(cmd).map_err(|error| TasksBuilderError::InvalidCommandSyntax {
                        task: target.to_owned(),
                        command: cmd.to_owned(),
                        position: match error.line_col {
                            LineColLocation::Pos((line, col)) => format!("{line}:{col}"),
                            LineColLocation::Span((line, col), _) => format!("{line}:{col}"),
                        },
                    })?;

                for pipeline in command_line.iter() {
                    match pipeline {
                        Pipeline::Start(commands) => {
                            let mut allow_next_sequence = true;

                            for sequence in commands.iter() {
                                match sequence {
                                    // If only env vars, allow it, otherwise it's a
                                    // multi-command and we shouldn't allow it
                                    Sequence::Start(command) | Sequence::Then(command) => {
                                        if !allow_next_sequence {
                                            return Err(
                                                TasksBuilderError::UnsupportedCommandSyntax {
                                                    task: target.to_owned(),
                                                }
                                                .into(),
                                            );
                                        }

                                        allow_next_sequence = command
                                            .iter()
                                            .all(|arg| matches!(arg, Argument::EnvVar(_, _, _)));
                                        let mut extract_env = true;

                                        for arg in command.iter() {
                                            handle_arg(arg, extract_env);

                                            if extract_env
                                                && !matches!(arg, Argument::EnvVar(_, _, _))
                                            {
                                                extract_env = false;
                                            }
                                        }
                                    }
                                    // Capture anything after `--`
                                    Sequence::Passthrough(command) => {
                                        handle_arg(
                                            &Argument::Value(Value::Unquoted("--".into())),
                                            false,
                                        );

                                        for arg in command.iter() {
                                            handle_arg(arg, false);
                                        }
                                    }
                                    Sequence::Stop(term) => {
                                        if term == ";" {
                                            // Allow
                                        } else {
                                            return Err(
                                                TasksBuilderError::UnsupportedCommandSyntax {
                                                    task: target.to_owned(),
                                                }
                                                .into(),
                                            );
                                        }
                                    }
                                    _ => {
                                        return Err(TasksBuilderError::UnsupportedCommandSyntax {
                                            task: target.to_owned(),
                                        }
                                        .into());
                                    }
                                };
                            }
                        }
                        _ => {
                            return Err(TasksBuilderError::UnsupportedCommandSyntax {
                                task: target.to_owned(),
                            }
                            .into());
                        }
                    };
                }

                if !args.is_empty() {
                    res.args = Some(args);
                }

                if !env.is_empty() {
                    res.env = Some(env);
                }

                Ok(res)
            }
        }
    }

    fn get_command_line(
        &self,
        target: &Target,
        config: &TaskConfig,
    ) -> miette::Result<Option<CommandLineParseResult>> {
        if config.script.is_some() {
            return Ok(None);
        }

        let parse_result = self.parse_command_line(target, &config.command)?;
        let mut command_line = CommandLineParseResult::default();

        if let Some(mut args) = parse_result.args {
            command_line.command = Some(args.remove(0));
            command_line.args.get_or_insert_default().extend(args);
        }

        if let Some(env) = parse_result.env {
            command_line.env.get_or_insert_default().extend(env);
        }

        if parse_result.requires_shell {
            command_line.requires_shell = true;
        }

        if config.args != TaskArgs::Noop {
            let parse_result = self.parse_command_line(target, &config.args)?;

            if let Some(args) = parse_result.args {
                command_line.args.get_or_insert_default().extend(args);
            }

            if let Some(env) = parse_result.env {
                command_line.env.get_or_insert_default().extend(env);
            }

            if parse_result.requires_shell {
                command_line.requires_shell = true;
            }
        }

        Ok(Some(command_line))
    }

    fn get_config_inherit_chain(&self, id: &Id) -> miette::Result<Vec<ConfigChain<'_>>> {
        let stack = extract_config(id, &self.local_tasks, &self.global_tasks)?;

        Ok(stack)
    }

    fn get_task_options_from_preset(
        &self,
        preset: Option<TaskPreset>,
        state: &mut TaskState,
    ) -> TaskOptions {
        if preset.is_some() {
            state.set_run_in_ci = true;
        }

        match preset {
            Some(TaskPreset::Utility) => TaskOptions {
                cache: TaskOptionCache::Enabled(false),
                interactive: true,
                output_style: Some(TaskOutputStyle::Stream),
                persistent: false,
                run_in_ci: TaskOptionRunInCI::Skip,
                ..Default::default()
            },
            Some(TaskPreset::Server) => TaskOptions {
                cache: TaskOptionCache::Enabled(false),
                output_style: Some(TaskOutputStyle::Stream),
                persistent: true,
                priority: TaskPriority::Low,
                run_in_ci: TaskOptionRunInCI::Enabled(false),
                ..Default::default()
            },
            _ => TaskOptions::default(),
        }
    }

    fn apply_filters_to_deps(&self, deps: Vec<TaskDependencyConfig>) -> Vec<TaskDependencyConfig> {
        let Some(filters) = &self.filters else {
            return deps;
        };

        deps.into_iter()
            .filter(|dep| !filters.exclude.contains(&dep.target.task_id))
            .map(|mut dep| {
                if let Some(new_task_id) = filters.rename.get(&dep.target.task_id) {
                    dep.target.id = Target::format(&dep.target.scope, new_task_id).into();
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
        index: usize,
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
            TaskMergeStrategy::Preserve => {
                if index == 0 {
                    next
                } else {
                    base
                }
            }
            TaskMergeStrategy::Replace => next,
        }
    }

    fn merge_index_map<K, V>(
        &self,
        base: IndexMap<K, V>,
        next: IndexMap<K, V>,
        strategy: TaskMergeStrategy,
        index: usize,
    ) -> IndexMap<K, V>
    where
        K: Eq + Hash,
    {
        match strategy {
            TaskMergeStrategy::Append => {
                if next.is_empty() {
                    return base;
                }

                let mut map = IndexMap::default();
                map.extend(base);
                map.extend(next);
                map
            }
            TaskMergeStrategy::Prepend => {
                if next.is_empty() {
                    return base;
                }

                let mut map = IndexMap::default();
                map.extend(next);
                map.extend(base);
                map
            }
            TaskMergeStrategy::Preserve => {
                if index == 0 {
                    next
                } else {
                    base
                }
            }
            TaskMergeStrategy::Replace => next,
        }
    }

    fn merge_vec<T: Eq + std::fmt::Debug>(
        &self,
        base: Vec<T>,
        next: Vec<T>,
        strategy: TaskMergeStrategy,
        index: usize,
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
            TaskMergeStrategy::Preserve => {
                if index == 0 {
                    list.extend(next);
                } else {
                    list.extend(base);
                }
            }
            TaskMergeStrategy::Replace => {
                list.extend(next);
            }
        }

        list
    }
}
