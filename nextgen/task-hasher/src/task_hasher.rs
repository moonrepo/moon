use moon_config::{HasherConfig, HasherWalkStrategy};
use moon_task::Task;
use moon_vcs::BoxedVcs;
use rustc_hash::FxHashSet;
use starbase_utils::glob;
use std::path::{Path, PathBuf};

pub struct TaskHasher<'task> {
    files_to_hash: FxHashSet<PathBuf>,

    task: &'task Task,
    project_root: &'task Path,
    project_source: &'task str,
    vcs: &'task BoxedVcs,
    workspace_root: &'task Path,
}

impl<'task> TaskHasher<'task> {
    // Hash all inputs for a task, but exclude outputs  and moon specific configuration files!
    pub async fn aggregate_inputs(&mut self, hasher_config: &HasherConfig) -> miette::Result<()> {
        if !self.task.input_files.is_empty() {
            for input in &self.task.input_files {
                self.files_to_hash
                    .insert(input.to_path(self.workspace_root));
            }
        }

        if !self.task.input_globs.is_empty() {
            let use_globs = self.project_root == self.workspace_root
                || matches!(hasher_config.walk_strategy, HasherWalkStrategy::Glob);

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

        Ok(())
    }
}
