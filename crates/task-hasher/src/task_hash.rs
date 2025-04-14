use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::OutputPath;
use moon_hash::hash_content;
use moon_project::Project;
use moon_task::{Target, Task};
use std::collections::BTreeMap;

hash_content!(
    pub struct TaskHash<'task> {
        // Task option `cacheKey`
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_key: Option<&'task str>,

        // Task `command`
        pub command: &'task str,

        // Task `args`
        pub args: Vec<&'task str>,

        // Task `deps` mapped to their hash
        pub deps: BTreeMap<&'task Target, String>,

        // Environment variables
        pub env: BTreeMap<&'task str, &'task str>,

        // Input files and globs mapped to a unique hash
        pub inputs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // Input environment variables
        pub input_env: BTreeMap<&'task str, String>,

        // Relative output paths
        pub outputs: Vec<&'task OutputPath>,

        // Project `dependsOn`
        pub project_deps: Vec<&'task Id>,

        // Task `script`
        #[serde(skip_serializing_if = "Option::is_none")]
        pub script: Option<&'task str>,

        // Task `target`
        pub target: &'task Target,

        // Task `toolchains`
        pub toolchains: Vec<&'task Id>,

        // Bump this to invalidate all caches
        pub version: String,
    }
);

impl<'task> TaskHash<'task> {
    pub fn new(project: &'task Project, task: &'task Task) -> Self {
        Self {
            cache_key: task.options.cache_key.as_deref(),
            command: &task.command,
            args: task.args.iter().map(|a| a.as_str()).collect(),
            deps: BTreeMap::new(),
            env: task
                .env
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
            inputs: BTreeMap::new(),
            input_env: BTreeMap::new(),
            outputs: task.outputs.iter().collect(),
            project_deps: project.get_dependency_ids(),
            script: task.script.as_deref(),
            target: &task.target,
            toolchains: task.toolchains.iter().collect(),
            // 1 - Original implementation
            // 2 - New task runner crate, tarball structure changed
            // 3 - New action pipeline
            version: "3".into(),
        }
    }
}
