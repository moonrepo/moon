use moon_config2::{
    Portable, PortablePath, TaskMergeStrategy, TaskOptionAffectedFiles, TaskOptionEnvFile,
    TaskOptionsConfig, TaskOutputStyle,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptions {
    pub affected_files: Option<TaskOptionAffectedFiles>,

    pub cache: bool,

    pub env_file: Option<String>,

    pub merge_args: TaskMergeStrategy,

    pub merge_deps: TaskMergeStrategy,

    pub merge_env: TaskMergeStrategy,

    pub merge_inputs: TaskMergeStrategy,

    pub merge_outputs: TaskMergeStrategy,

    pub output_style: Option<TaskOutputStyle>,

    pub persistent: bool,

    pub retry_count: u8,

    pub run_deps_in_parallel: bool,

    pub run_in_ci: bool,

    pub run_from_workspace_root: bool,

    pub shell: bool,
}

impl Default for TaskOptions {
    fn default() -> Self {
        TaskOptions {
            affected_files: None,
            cache: true,
            env_file: None,
            merge_args: TaskMergeStrategy::Append,
            merge_deps: TaskMergeStrategy::Append,
            merge_env: TaskMergeStrategy::Append,
            merge_inputs: TaskMergeStrategy::Append,
            merge_outputs: TaskMergeStrategy::Append,
            output_style: None,
            persistent: false,
            retry_count: 0,
            run_deps_in_parallel: true,
            run_in_ci: true,
            run_from_workspace_root: false,
            shell: true,
        }
    }
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
        if let Some(affected_files) = &config.affected_files {
            self.affected_files = Some(affected_files.to_owned());
        }

        if let Some(cache) = &config.cache {
            self.cache = *cache;
        }

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

        if let Some(persistent) = &config.persistent {
            self.persistent = *persistent;
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

        if let Some(shell) = &config.shell {
            self.shell = *shell;
        }
    }

    pub fn from_config(config: TaskOptionsConfig, is_local: bool) -> TaskOptions {
        TaskOptions {
            affected_files: config.affected_files,
            cache: config.cache.unwrap_or(!is_local),
            env_file: config
                .env_file
                .map(|env_file| env_file.to_option().unwrap()),
            merge_args: config.merge_args.unwrap_or_default(),
            merge_deps: config.merge_deps.unwrap_or_default(),
            merge_env: config.merge_env.unwrap_or_default(),
            merge_inputs: config.merge_inputs.unwrap_or_default(),
            merge_outputs: config.merge_outputs.unwrap_or_default(),
            output_style: config
                .output_style
                .or_else(|| is_local.then_some(TaskOutputStyle::Stream)),
            persistent: config.persistent.unwrap_or(is_local),
            retry_count: config.retry_count.unwrap_or_default(),
            run_deps_in_parallel: config.run_deps_in_parallel.unwrap_or(true),
            run_in_ci: config.run_in_ci.unwrap_or(!is_local),
            run_from_workspace_root: config.run_from_workspace_root.unwrap_or_default(),
            shell: config.shell.unwrap_or(true),
        }
    }

    pub fn to_config(&self) -> TaskOptionsConfig {
        let default_options = TaskOptions::default();
        let mut config = TaskOptionsConfig::default();

        // Skip merge options until we need them

        if let Some(affected_files) = &self.affected_files {
            config.affected_files = Some(affected_files.to_owned());
        }

        if self.cache != default_options.cache {
            config.cache = Some(self.cache);
        }

        if let Some(env_file) = &self.env_file {
            config.env_file = Some(if env_file == ".env" {
                TaskOptionEnvFile::Enabled(true)
            } else {
                TaskOptionEnvFile::File(PortablePath::from_str(env_file).unwrap())
            });
        }

        if let Some(output_style) = &self.output_style {
            config.output_style = Some(output_style.clone());
        }

        if self.persistent != default_options.persistent {
            config.persistent = Some(self.persistent);
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

        if self.shell != default_options.shell {
            config.shell = Some(self.shell);
        }

        config
    }
}
