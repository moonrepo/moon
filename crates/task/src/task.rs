use monolith_config::{TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TaskOptions {
    pub merge_strategy: TaskMergeStrategy,

    pub retry_count: u8,
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
        if let Some(merge_strategy) = &config.merge_strategy {
            self.merge_strategy = merge_strategy.clone();
        }

        if let Some(retry_count) = &config.retry_count {
            self.retry_count = *retry_count;
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Task {
    pub args: Vec<String>,

    pub command: String,

    pub inputs: Vec<String>,

    pub name: String,

    pub options: TaskOptions,

    pub outputs: Vec<String>,

    pub type_of: TaskType,
}

impl Task {
    pub fn from_config(name: &str, config: &TaskConfig) -> Self {
        let config_options = config
            .options
            .as_ref()
            .map_or_else(TaskOptionsConfig::default, |v| v.clone());

        let options = TaskOptions {
            merge_strategy: config_options
                .merge_strategy
                .as_ref()
                .map_or_else(|| TaskMergeStrategy::Append, |v| v.clone()),
            retry_count: config_options.retry_count.unwrap_or_default(),
        };

        Task {
            args: config.args.as_ref().map_or_else(Vec::new, |v| v.clone()),
            command: config.command.clone(),
            inputs: config.inputs.as_ref().map_or_else(Vec::new, |v| v.clone()),
            name: name.to_owned(),
            options,
            outputs: config.outputs.as_ref().map_or_else(Vec::new, |v| v.clone()),
            type_of: config
                .type_of
                .as_ref()
                .map_or_else(TaskType::default, |v| v.clone()),
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
