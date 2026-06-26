#![allow(unused)]

use bazel_remote_apis::build::bazel::remote::execution::v2::{Action, ActionResult};
use moon_blob::Blob;
use moon_common::is_ci;
use moon_config::RemoteConfig;
use moon_hash::Digest;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tracing::instrument;

static INSTANCE: OnceLock<Arc<RemoteService>> = OnceLock::new();

pub struct RemoteService {
    pub config: RemoteConfig,
    pub workspace_root: PathBuf,
    cache_enabled: bool,
}

impl RemoteService {
    pub fn session() -> Option<Arc<RemoteService>> {
        INSTANCE.get().cloned()
    }

    pub fn is_enabled() -> bool {
        INSTANCE.get().is_some_and(|remote| remote.cache_enabled)
    }

    #[instrument]
    pub async fn connect(config: &RemoteConfig, workspace_root: &Path) -> miette::Result<()> {
        unreachable!()
    }

    pub async fn validate_capabilities(&mut self) -> miette::Result<()> {
        unreachable!()
    }

    pub fn can_download(&self) -> bool {
        self.cache_enabled
    }

    pub fn can_upload(&self) -> bool {
        self.cache_enabled && (is_ci() || !self.config.cache.local_read_only)
    }

    pub fn get_max_batch_size(&self) -> i64 {
        0
    }

    #[instrument(skip(self))]
    pub async fn is_action_cached(
        &self,
        action_digest: &Digest,
    ) -> miette::Result<Option<ActionResult>> {
        unreachable!()
    }

    #[instrument(skip(self, _action, blob))]
    pub async fn save_action(&self, _action: Action, blob: Blob) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self, result, blobs))]
    pub async fn save_action_result(
        &self,
        action_digest: &Digest,
        mut result: ActionResult,
        blobs: Vec<Blob>,
    ) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self, result))]
    pub async fn restore_action_result(
        &self,
        action_digest: &Digest,
        result: &mut ActionResult,
    ) -> miette::Result<bool> {
        unreachable!()
    }

    #[instrument(skip(self))]
    pub async fn wait_for_requests(&self) {
        unreachable!()
    }
}
