use miette::IntoDiagnostic;
use moon_cache_item::cache_item;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::{Blob, ContentHash, OutputDigests};
use moon_task::Task;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

cache_item!(
    pub struct TaskRunCacheState {
        pub exit_code: i32,
        pub hash: String,
        pub last_run_time: u128,
        pub target: String,

        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        pub output_hashes: BTreeMap<WorkspaceRelativePathBuf, ContentHash>,
    }
);

pub struct TaskRunState<'task> {
    task: &'task Task,
}

impl TaskRunState<'_> {
    pub async fn compute_outputs(
        &mut self,
        workspace_root: &Path,
    ) -> miette::Result<OutputDigests> {
        let mut set = JoinSet::<miette::Result<(PathBuf, Blob)>>::new();
        let mut outputs = OutputDigests::default();

        for path in self.task.get_output_files(workspace_root, true)? {
            set.spawn_blocking(move || {
                let blob = Blob::from_file(&path)?;
                Ok((path, blob))
            });
        }

        while let Some(result) = set.join_next().await {
            let (path, blob) = result.into_diagnostic()??;
            outputs.blobs.insert(path, blob);
        }

        Ok(outputs)
    }
}
