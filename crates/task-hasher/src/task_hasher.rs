use crate::task_hash::TaskHash;
use crate::task_hasher_error::TaskHasherError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::color;
use moon_common::path::{PathExt, WorkspaceRelativePath, WorkspaceRelativePathBuf};
use moon_config::{HasherConfig, HasherWalkStrategy, Input, ProjectInput};
use moon_env_var::GlobalEnvBag;
use moon_feature_flags::glob_walk_with_options;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::{Target, Task};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob::{GlobSet, GlobWalkOptions};
use std::path::PathBuf;
use tracing::{trace, warn};

// Hash all inputs for a task, but exclude outputs and moon specific configuration files!
pub struct TaskHasher<'task> {
    pub app_context: &'task AppContext,
    pub project_graph: &'task ProjectGraph,
    pub project: &'task Project,
    pub task: &'task Task,
    pub hasher_config: &'task HasherConfig,

    content: TaskHash<'task>,
}

impl<'task> TaskHasher<'task> {
    pub fn new(
        app_context: &'task AppContext,
        project_graph: &'task ProjectGraph,
        project: &'task Project,
        task: &'task Task,
        hasher_config: &'task HasherConfig,
    ) -> Self {
        Self {
            app_context,
            project,
            project_graph,
            task,
            hasher_config,
            content: TaskHash::new(project, task),
        }
    }

    pub fn hash(mut self) -> TaskHash<'task> {
        // Ensure hashing is deterministic
        self.content.args.sort();
        self.content.project_deps.sort();

        // Consume the hasher and return the content
        self.content
    }

    pub fn hash_args(&mut self, args: impl IntoIterator<Item = &'task String>) {
        for arg in args {
            self.content.args.push(arg);
        }
    }

    pub fn hash_deps(&mut self, deps: impl IntoIterator<Item = (&'task Target, String)>) {
        self.content.deps.extend(deps);
    }

    pub fn hash_env(&mut self, env: impl IntoIterator<Item = (&'task String, &'task String)>) {
        for (key, value) in env {
            self.content.env.insert(key.as_ref(), value.as_ref());
        }
    }

    pub async fn hash_inputs(&mut self) -> miette::Result<()> {
        let absolute_inputs = self.aggregate_inputs().await?;
        let processed_inputs = self.process_inputs(absolute_inputs)?;

        if !processed_inputs.is_empty() && self.app_context.vcs.is_enabled() {
            let files = processed_inputs.into_iter().collect::<Vec<_>>();

            self.content.inputs = self.app_context.vcs.get_file_hashes(&files, true).await?;
        }

        if !self.task.input_env.is_empty() {
            let bag = GlobalEnvBag::instance();

            for input in &self.task.input_env {
                self.content
                    .input_env
                    .insert(input, bag.get(input).unwrap_or_default());
            }
        }

        Ok(())
    }

    async fn aggregate_inputs(&mut self) -> miette::Result<FxHashSet<PathBuf>> {
        let mut files = FxHashSet::default();
        let vcs_enabled = self.app_context.vcs.is_enabled();
        let workspace_root = &self.app_context.workspace_root;

        if !self.task.input_files.is_empty() {
            for file in self.task.input_files.keys() {
                files.insert(file.to_logical_path(workspace_root));
            }
        }

        if !self.task.input_globs.is_empty() {
            let use_globs = &self.project.root == workspace_root
                || matches!(self.hasher_config.walk_strategy, HasherWalkStrategy::Glob)
                || !vcs_enabled;

            // Collect inputs by walking and globbing the file system
            if use_globs {
                files.extend(self.task.get_input_files_with_globs(
                    workspace_root,
                    self.task.input_globs.iter().collect(),
                )?);
            }
            // Collect inputs by querying VCS which is faster than globbing
            else {
                for file in self
                    .app_context
                    .vcs
                    .get_file_tree(&self.project.source)
                    .await?
                {
                    files.insert(file.to_logical_path(workspace_root));
                }

                // However that completely ignores workspace level globs,
                // so we must still manually glob those here!
                let workspace_globs = self
                    .task
                    .input_globs
                    .iter()
                    .filter(|(glob, _)| !glob.as_str().starts_with(self.project.source.as_str()))
                    .collect::<FxHashMap<_, _>>();

                if !workspace_globs.is_empty() {
                    files.extend(
                        self.task
                            .get_input_files_with_globs(workspace_root, workspace_globs)?,
                    );
                }
            }
        }

        for input in &self.task.inputs {
            if let Input::Project(inner) = &input
                && !inner.is_all_deps()
            {
                files.extend(self.aggregate_inputs_from_project(inner).await?);
            }
        }

        // Include local file changes so that development builds work.
        // Also run this LAST as it should take highest precedence!
        if vcs_enabled {
            for local_file in self.app_context.vcs.get_touched_files().await?.all() {
                let abs_file = local_file.to_logical_path(workspace_root);

                // Deleted files are listed in `git status` but are
                // not valid inputs, so avoid hashing them!
                if abs_file.exists() {
                    if local_file.starts_with(&self.project.source) {
                        files.insert(abs_file);
                    }
                } else {
                    files.remove(&abs_file);
                }
            }
        }

        Ok(files)
    }

    async fn aggregate_inputs_from_project(
        &mut self,
        input: &ProjectInput,
    ) -> miette::Result<FxHashSet<PathBuf>> {
        let mut files = FxHashSet::default();
        let project = self.project_graph.get(&input.project)?;

        if let Some(group_id) = &input.group {
            files.extend(project.get_file_group(group_id)?.walk_absolute(
                false,
                &self.app_context.workspace_root,
                false,
            )?)
        } else {
            let default_globs = vec!["**/*".to_owned()];

            files.extend(glob_walk_with_options(
                &project.root,
                if input.filter.is_empty() {
                    &default_globs
                } else {
                    &input.filter
                },
                GlobWalkOptions::default().cache().files(),
            )?);
        }

        Ok(files)
    }

    fn is_valid_input_source(
        &self,
        sources_globset: &GlobSet,
        workspace_relative_path: &WorkspaceRelativePath,
    ) -> bool {
        // Don't invalidate existing hashes when moon.* changes
        // as we already hash the contents of each task!
        if self.task.state.default_inputs
            && (workspace_relative_path.ends_with("moon.yml")
                || workspace_relative_path.ends_with("moon.pkl"))
        {
            return false;
        }

        // Remove outputs first
        if sources_globset.is_negated(workspace_relative_path.as_str()) {
            return false;
        }

        for output in self.task.output_files.keys() {
            if workspace_relative_path == output || workspace_relative_path.starts_with(output) {
                return false;
            }
        }

        // Input may be in another project, or the workspace,
        // so the task-level globs should not apply to it
        if !workspace_relative_path.starts_with(&self.project.source) {
            return true;
        }

        // Filter inputs second
        self.task.input_files.contains_key(workspace_relative_path)
            || sources_globset.matches(workspace_relative_path.as_str())
    }

    fn process_inputs(
        &mut self,
        inputs: FxHashSet<PathBuf>,
    ) -> miette::Result<FxHashSet<WorkspaceRelativePathBuf>> {
        let mut files = FxHashSet::default();
        let ignore = GlobSet::new(&self.hasher_config.ignore_patterns)?;
        let ignore_missing = GlobSet::new(&self.hasher_config.ignore_missing_patterns)?;
        let globset = self.task.create_globset()?;
        let has_globs = !self.task.input_globs.is_empty() || !self.task.output_globs.is_empty();

        for abs_path in inputs {
            // We need to use relative paths from the workspace root
            // so that it works the same across all machines
            let rel_path = abs_path
                .relative_to(&self.app_context.workspace_root)
                .into_diagnostic()?;

            if has_globs && !self.is_valid_input_source(&globset, &rel_path) {
                continue;
            }

            if !abs_path.exists() {
                if let Some(params) = self.task.input_files.get(&rel_path) {
                    match params.optional {
                        Some(true) => continue,
                        Some(false) => {
                            return Err(TaskHasherError::MissingInputFile {
                                path: rel_path.to_string(),
                                target: self.task.target.clone(),
                            }
                            .into());
                        }
                        _ => {}
                    };
                }

                if self.hasher_config.warn_on_missing_inputs
                    && (self.hasher_config.ignore_missing_patterns.is_empty()
                        || !ignore_missing.is_match(abs_path))
                {
                    warn!(
                        "Attempted to hash input {} but it does not exist, skipping",
                        color::rel_path(&rel_path),
                    );
                }

                continue;
            }

            if !abs_path.is_file() {
                warn!(
                    "Attempted to hash input {} but only files can be hashed, try using a glob instead",
                    color::rel_path(&rel_path),
                );

                continue;
            }

            if ignore.is_match(abs_path) {
                trace!(
                    "Not hashing input {} as it matches an ignore pattern",
                    color::rel_path(&rel_path),
                );
            } else {
                files.insert(rel_path);
            }
        }

        Ok(files)
    }
}
