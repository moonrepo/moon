use monolith_config::{TaskConfig, TaskOptionsConfig, TaskType};

#[derive(Debug)]
pub struct TaskOptions {
    pub retry_count: u8,
}

#[derive(Debug)]
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
}
