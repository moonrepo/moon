use moon_common::is_ci;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{HasherConfig, HasherWalkStrategy};
use moon_task::Task;
use moon_vcs::BoxedVcs;
use rustc_hash::FxHashSet;
use starbase_utils::glob;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct TaskHasher<'task> {
    pub hasher_config: &'task HasherConfig,
    pub task: &'task Task,
    pub project_root: &'task Path,
    pub project_source: &'task str,
    pub vcs: &'task BoxedVcs,
    pub workspace_root: &'task Path,

    files_to_hash: FxHashSet<PathBuf>,
}

impl<'task> TaskHasher<'task> {
    // Hash all inputs for a task, but exclude outputs  and moon specific configuration files!
    pub async fn aggregate_inputs(&mut self) -> miette::Result<()> {
        if !self.task.input_files.is_empty() {
            for input in &self.task.input_files {
                self.files_to_hash
                    .insert(input.to_path(self.workspace_root));
            }
        }

        if !self.task.input_globs.is_empty() {
            let use_globs = self.project_root == self.workspace_root
                || matches!(self.hasher_config.walk_strategy, HasherWalkStrategy::Glob);

            // Collect inputs by walking and globbing the file system
            if use_globs {
                self.files_to_hash.extend(glob::walk_files(
                    self.workspace_root,
                    &self.task.input_globs,
                )?);

                // Collect inputs by querying VCS
            } else {
                // Using VCS to collect inputs in a project is faster than globbing
                for file in self.vcs.get_file_tree(self.project_source).await? {
                    self.files_to_hash.insert(file.to_path(self.workspace_root));
                }

                // However that completely ignores workspace level globs,
                // so we must still manually glob those here!
                let workspace_globs = self
                    .task
                    .input_globs
                    .iter()
                    .filter(|g| !g.starts_with(self.project_source))
                    .collect::<Vec<_>>();

                if !workspace_globs.is_empty() {
                    self.files_to_hash
                        .extend(glob::walk_files(self.workspace_root, workspace_globs)?);
                }
            }
        }

        // Include local file changes so that development builds work.
        // Also run this LAST as it should take highest precedence!
        if !is_ci() {
            for local_file in self.vcs.get_touched_files().await?.all() {
                let local_file = local_file.to_path(self.workspace_root);

                // Deleted files are listed in `git status` but are
                // not valid inputs, so avoid hashing them!
                if local_file.exists() {
                    self.files_to_hash.insert(local_file);
                }
            }
        }

        Ok(())
    }

    pub fn process_files(&mut self) -> miette::Result<()> {
        Ok(())
    }

    pub fn generate_hashes(self) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        Ok(BTreeMap::default())
    }
}
