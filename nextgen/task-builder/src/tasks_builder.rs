#![allow(dead_code)]

use moon_args::{split_args, ArgsSplitError};
use moon_common::{color, Id};
use moon_config::{
    InheritedTasksConfig, InputPath, ProjectConfig, ProjectWorkspaceInheritedTasksConfig,
    TaskCommandArgs, TaskConfig, TaskMergeStrategy, TaskOutputStyle, TaskType,
};
use moon_task2::{Task, TaskOptions};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::Path;
use tracing::debug;

pub struct TasksBuilder<'proj> {
    project_root: &'proj Path,
    workspace_root: &'proj Path,

    task_ids: FxHashSet<&'proj Id>,
    global_tasks: FxHashMap<&'proj Id, &'proj TaskConfig>,
    local_tasks: FxHashMap<&'proj Id, &'proj TaskConfig>,
}

impl<'proj> TasksBuilder<'proj> {
    pub fn new(project_root: &'proj Path, workspace_root: &'proj Path) -> Self {
        Self {
            project_root,
            workspace_root,
            task_ids: FxHashSet::default(),
            global_tasks: FxHashMap::default(),
            local_tasks: FxHashMap::default(),
        }
    }

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

        debug!("Filtering global tasks");

        for (task_id, task_config) in &global_config.tasks {
            // None = Include all
            // [] = Include none
            // ["a"] = Include "a"
            if !include_all {
                if include_set.is_empty() {
                    debug!("Not inheriting global tasks, empty include filter");

                    break;
                } else if !include_set.contains(task_id) {
                    debug!(
                        "Not inheriting global task {}, not included",
                        color::id(task_id)
                    );

                    continue;
                }
            }

            // None, [] = Exclude none
            // ["a"] = Exclude "a"
            if !exclude.is_empty() && exclude.contains(&&task_id) {
                debug!(
                    "Not inheriting global task {}, excluded",
                    color::id(task_id)
                );

                continue;
            }

            let task_key = if let Some(renamed_task_id) = rename.get(task_id) {
                debug!(
                    "Inheriting global task {} and renaming to {}",
                    color::id(task_id),
                    color::id(renamed_task_id)
                );

                renamed_task_id
            } else {
                debug!("Inheriting global task {}", color::id(task_id));

                task_id
            };

            self.global_tasks.insert(task_key, task_config);
            self.task_ids.insert(task_key);
        }

        self
    }

    pub fn load_local_tasks(&mut self, local_config: &'proj ProjectConfig) -> &mut Self {
        self.local_tasks.extend(&local_config.tasks);

        for id in local_config.tasks.keys() {
            self.task_ids.insert(id);
        }

        self
    }

    pub fn build(self) -> miette::Result<BTreeMap<Id, Task>> {
        let mut tasks = BTreeMap::new();

        for id in &self.task_ids {
            tasks.insert((*id).to_owned(), self.build_task(id)?);
        }

        Ok(tasks)
    }

    fn build_task(&self, id: &Id) -> miette::Result<Task> {
        let mut task = Task::default();
        let mut configs = vec![];

        if let Some(config) = self.global_tasks.get(id) {
            configs.push(*config);
        }

        if let Some(config) = self.local_tasks.get(id) {
            configs.push(*config);
        }

        // Determine command and args before building options and the task,
        // as we need to figure out if we're running locally or not.
        let mut is_local = false;
        let mut args = vec![];

        for config in &configs {
            let (command, base_args) = self.get_command_and_args(config)?;

            if let Some(command) = command {
                task.command = command;
            }

            // Set later after we have a merge strategy
            args.extend(base_args);

            if let Some(local) = config.local {
                is_local = local;
            } else {
                is_local =
                    task.command == "dev" || task.command == "serve" || task.command == "start";
            }
        }

        task.options = self.build_task_options(id, is_local)?;
        task.flags.local = is_local;

        // Finally build the task itself, while applying our complex inputs logic,
        // and inheriting implicit deps/inputs from the global config.
        let mut configured_inputs = 0;
        let mut has_configured_inputs = false;

        if !args.is_empty() {
            task.args = self.merge_vec(task.args, args, task.options.merge_args);
        }

        for config in configs {
            if !config.deps.is_empty() {
                task.deps =
                    self.merge_vec(task.deps, config.deps.to_owned(), task.options.merge_deps);
            }

            if !config.env.is_empty() {
                task.env = self.merge_map(task.env, config.env.to_owned(), task.options.merge_env);
            }

            // Inherit global inputs as normal inputs, but do not consider them a configured input
            if !config.global_inputs.is_empty() {
                task.inputs = self.merge_vec(
                    task.inputs,
                    config.global_inputs.to_owned(),
                    task.options.merge_inputs,
                );
            }

            // Inherit local inputs, which are used configured, and keep track of the total
            if let Some(inputs) = &config.inputs {
                configured_inputs += inputs.len();
                has_configured_inputs = true;

                task.inputs =
                    self.merge_vec(task.inputs, inputs.to_owned(), task.options.merge_inputs);
            }

            if let Some(outputs) = &config.outputs {
                task.outputs =
                    self.merge_vec(task.outputs, outputs.to_owned(), task.options.merge_outputs);
            }

            if !config.platform.is_unknown() {
                task.platform = config.platform;
            }
        }

        // Inputs are tricky, as they come from many sources. We need to ensure that user configured
        // inputs are handled explicitly, while globally inherited sources are handled implicitly.
        if configured_inputs == 0 {
            if has_configured_inputs {
                task.flags.empty_inputs = true;
            } else {
                task.inputs.push(InputPath::ProjectGlob("**/*".into()));
            }
        }

        // Always break cache if a core configuration changes
        task.inputs
            .push(InputPath::WorkspaceGlob(".moon/*.yml".into()));

        // And lastly, before we return the task and options, we should finalize all necessary
        // fields and populate/calculate with values.
        if task.command.is_empty() {
            task.command = "noop".into();
        }

        task.id = id.to_owned();
        // task.target; TODO

        task.type_of = if !task.outputs.is_empty() {
            TaskType::Build
        } else if is_local {
            TaskType::Run
        } else {
            TaskType::Test
        };

        Ok(task)
    }

    fn build_task_options(&self, id: &Id, is_local: bool) -> miette::Result<TaskOptions> {
        let mut options = TaskOptions {
            cache: !is_local,
            output_style: is_local.then_some(TaskOutputStyle::Stream),
            persistent: is_local,
            run_in_ci: !is_local,
            ..TaskOptions::default()
        };

        let mut configs = vec![];

        if let Some(config) = self.global_tasks.get(id) {
            configs.push(&config.options);
        }

        if let Some(config) = self.local_tasks.get(id) {
            configs.push(&config.options);
        }

        for config in configs {
            if let Some(affected_files) = &config.affected_files {
                options.affected_files = Some(affected_files.to_owned());
            }

            if let Some(cache) = &config.cache {
                options.cache = *cache;
            }

            if let Some(env_file) = &config.env_file {
                options.env_file = env_file.to_option();
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
                options.shell = *shell;
            }
        }

        Ok(options)
    }

    fn get_command_and_args(
        &self,
        config: &TaskConfig,
    ) -> Result<(Option<String>, Vec<String>), ArgsSplitError> {
        let mut command = None;
        let mut args = vec![];

        let mut cmd_list = match &config.command {
            TaskCommandArgs::None => vec![],
            TaskCommandArgs::String(cmd_string) => split_args(cmd_string)?,
            TaskCommandArgs::Sequence(cmd_args) => cmd_args.clone(),
        };

        if !cmd_list.is_empty() {
            command = Some(cmd_list.remove(0));
            args.extend(cmd_list);
        }

        match &config.args {
            TaskCommandArgs::None => {}
            TaskCommandArgs::String(args_string) => args.extend(split_args(args_string)?),
            TaskCommandArgs::Sequence(args_list) => args.extend(args_list.clone()),
        }

        Ok((command, args))
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
                let mut map = FxHashMap::default();
                map.extend(base);
                map.extend(next);
                map
            }
            TaskMergeStrategy::Prepend => {
                let mut map = FxHashMap::default();
                map.extend(next);
                map.extend(base);
                map
            }
            TaskMergeStrategy::Replace => next,
        }
    }

    fn merge_vec<T>(&self, base: Vec<T>, next: Vec<T>, strategy: TaskMergeStrategy) -> Vec<T> {
        let mut list: Vec<T> = vec![];

        match strategy {
            TaskMergeStrategy::Append => {
                list.extend(base);
                list.extend(next);
            }
            TaskMergeStrategy::Prepend => {
                list.extend(next);
                list.extend(base);
            }
            TaskMergeStrategy::Replace => {
                list.extend(next);
            }
        }

        list
    }
}
