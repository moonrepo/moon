use crate::errors::RunnerError;
use moon_action_context::TargetState;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::OutputPath;
use moon_hash::hash_content;
use moon_target::Target;
use moon_task::Task;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

hash_content!(
    pub struct TargetHasher<'task> {
        // Task `command`
        command: &'task str,

        // Task `args`
        args: Vec<&'task str>,

        // Task `deps` mapped to their hash
        deps: BTreeMap<&'task Target, &'task str>,

        // Environment variables
        env_vars: BTreeMap<&'task str, &'task str>,

        // Input files and globs mapped to a unique hash
        inputs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // Relative output paths
        outputs: Vec<&'task OutputPath>,

        // `moon.yml` `dependsOn`
        project_deps: Vec<&'task Id>,

        // Task `target`
        target: &'task Target,

        // Bump this to invalidate all caches
        version: String,
    }
);

impl<'task> TargetHasher<'task> {
    pub fn new(task: &'task Task) -> Self {
        TargetHasher {
            command: &task.command,
            args: task.args.iter().map(|a| a.as_str()).collect(),
            deps: BTreeMap::new(),
            env_vars: task
                .env
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
            inputs: BTreeMap::new(),
            outputs: task.outputs.iter().collect(),
            project_deps: Vec::new(),
            target: &task.target,
            version: "1".into(),
        }
    }

    /// Hash additional args outside of the provided task.
    pub fn hash_args(&mut self, passthrough_args: &'task [String]) {
        if !passthrough_args.is_empty() {
            for arg in passthrough_args {
                self.args.push(arg);
            }
        }
    }

    /// Hash a mapping of input file paths to unique file hashes.
    /// File paths *must* be relative from the workspace root.
    pub fn hash_inputs(&mut self, inputs: BTreeMap<WorkspaceRelativePathBuf, String>) {
        self.inputs.extend(inputs);
    }

    /// Hash `dependsOn` from the owning project.
    pub fn hash_project_deps(&mut self, deps: Vec<&'task Id>) {
        self.project_deps = deps;
        self.project_deps.sort();
    }

    /// Hash `deps` from a task and associate it with their current hash.
    pub fn hash_task_deps(
        &mut self,
        task: &'task Task,
        states: &'task FxHashMap<Target, TargetState>,
    ) -> miette::Result<()> {
        for dep in &task.deps {
            self.deps.insert(
                &dep.target,
                match states.get(&dep.target) {
                    Some(TargetState::Completed(hash)) => hash,
                    Some(TargetState::Passthrough) => "passthrough",
                    _ => {
                        return Err(RunnerError::MissingDependencyHash(
                            dep.target.id.to_owned(),
                            task.target.id.to_owned(),
                        )
                        .into());
                    }
                },
            );
        }

        Ok(())
    }
}
