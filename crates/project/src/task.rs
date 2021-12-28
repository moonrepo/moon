use monolith_config::{Target, TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TaskOptions {
    #[serde(rename = "mergeStrategy")]
    pub merge_strategy: TaskMergeStrategy,

    #[serde(rename = "retryCount")]
    pub retry_count: u8,

    #[serde(rename = "runInCi")]
    pub run_in_ci: bool,

    #[serde(rename = "runFromWorkspaceRoot")]
    pub run_from_workspace_root: bool,
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
        if let Some(merge_strategy) = &config.merge_strategy {
            self.merge_strategy = merge_strategy.clone();
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
pub struct Task {
    pub args: Vec<String>,

    pub command: String,

    pub depends_on: Vec<Target>,

    pub inputs: Vec<String>,

    pub name: String,

    pub options: TaskOptions,

    pub outputs: Vec<String>,

    #[serde(rename = "type")]
    pub type_of: TaskType,
}

impl Task {
    pub fn from_config(name: &str, config: &TaskConfig) -> Self {
        let cloned_config = config.clone();
        let cloned_options = cloned_config.options.unwrap_or_default();

        Task {
            args: cloned_config.args.unwrap_or_else(Vec::new),
            command: cloned_config.command,
            depends_on: cloned_config.depends_on.unwrap_or_else(Vec::new),
            inputs: cloned_config.inputs.unwrap_or_else(Vec::new),
            name: name.to_owned(),
            options: TaskOptions {
                merge_strategy: cloned_options
                    .merge_strategy
                    .unwrap_or(TaskMergeStrategy::Append),
                retry_count: cloned_options.retry_count.unwrap_or_default(),
                run_in_ci: cloned_options.run_in_ci.unwrap_or_default(),
                run_from_workspace_root: cloned_options.run_from_workspace_root.unwrap_or_default(),
            },
            outputs: cloned_config.outputs.unwrap_or_else(Vec::new),
            type_of: cloned_config.type_of.unwrap_or_default(),
        }
    }

    pub fn merge(&mut self, config: &TaskConfig) {
        // Merge options first incase the strategy has changed
        if let Some(options) = &config.options {
            self.options.merge(options);
        }

        // Then merge the actual task fields
        self.command = config.command.clone();

        if let Some(args) = &config.args {
            self.args = self.merge_string_vec(&self.args, args);
        }

        if let Some(depends_on) = &config.depends_on {
            self.depends_on = self.merge_string_vec(&self.depends_on, depends_on);
        }

        if let Some(inputs) = &config.inputs {
            self.inputs = self.merge_string_vec(&self.inputs, inputs);
        }

        if let Some(outputs) = &config.outputs {
            self.outputs = self.merge_string_vec(&self.outputs, outputs);
        }

        if let Some(type_of) = &config.type_of {
            self.type_of = type_of.clone();
        }
    }

    fn merge_string_vec(&self, base: &[String], next: &[String]) -> Vec<String> {
        let mut list: Vec<String> = vec![];

        // This is easier than .extend() as we need to clone the inner string
        let mut merge = |inner_list: &[String]| {
            for item in inner_list {
                list.push(item.clone());
            }
        };

        match self.options.merge_strategy {
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
