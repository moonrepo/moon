// Integration tests for the ArchiveTaskOutputs / HydrateTaskOutputs flows:
// spin up a real daemon over a UDS, drive it with a real client, and assert on
// the cache and the workspace rather than on the response alone.

#![cfg(unix)]

use moon_app_context::AppContext;
use moon_cache_storage::{Manifest, ManifestFile};
use moon_daemon_client::DaemonClient;
use moon_daemon_server::{DaemonService, DaemonState, serve_unix};
use moon_daemon_utils::endpoint::*;
use moon_hash::Digest;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};

/// The id the mocker's local storage backend registers itself under.
const LOCAL_BACKEND: &str = "local-cache";

struct TestDaemon {
    app_context: Arc<AppContext>,
    client: DaemonClient,
    sandbox: Sandbox,
    shutdown_tx: broadcast::Sender<()>,
}

impl TestDaemon {
    /// Start a daemon whose app context is shared with the test, so assertions
    /// read the same cache the handlers wrote to.
    async fn start() -> Self {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();

        let mocker = WorkspaceMocker::new(sandbox.path());
        let app_context = Arc::new(mocker.mock_app_context());

        let endpoint = get_endpoint(&daemon_dir);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let service = DaemonService::new(
            Arc::new(RwLock::new(DaemonState {
                app_context: Arc::clone(&app_context),
                workspace_graph: Arc::new(WorkspaceGraph::default()),
            })),
            endpoint.clone(),
            std::process::id(),
            shutdown_tx.clone(),
        );

        tokio::spawn(async move {
            let shutdown_signal = async move {
                let _ = shutdown_rx.recv().await;
            };

            serve_unix(&endpoint, service, shutdown_signal)
                .await
                .unwrap();
        });

        // Poll rather than sleep a fixed amount, so a slow bind doesn't flake.
        for _ in 0..100 {
            if DaemonClient::test_connection(&daemon_dir).await {
                break;
            }

            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let client = DaemonClient::connect(&daemon_dir).await.unwrap();

        Self {
            app_context,
            client,
            sandbox,
            shutdown_tx,
        }
    }

    async fn manifest_exists(&self, digest: &Digest) -> bool {
        self.app_context
            .cache_engine
            .storage
            .load_manifest(digest)
            .await
            .unwrap()
            .is_some()
    }

    async fn blob_exists(&self, digest: &Digest) -> bool {
        let backend = self.app_context.cache_engine.storage.get_backends()[0].clone();

        backend
            .find_missing_blobs(vec![digest.clone()])
            .await
            .unwrap()
            .is_empty()
    }

    /// The archive RPC acks before the work runs, so wait for the manifest to
    /// actually land instead of racing it.
    async fn wait_for_manifest(&self, digest: &Digest) -> bool {
        for _ in 0..100 {
            if self.manifest_exists(digest).await {
                return true;
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        false
    }

    /// Persist a manifest straight into storage, standing in for a previous run
    /// so hydration has something to find.
    async fn seed(&self, digest: &Digest, manifest: Manifest) {
        self.app_context
            .cache_engine
            .storage
            .archive_manifest(digest, manifest)
            .await
            .unwrap();

        self.app_context
            .cache_engine
            .storage
            .wait_for_background_tasks()
            .await
            .unwrap();
    }

    /// The manifest as a hydration source sees it: digests, no inline bytes.
    async fn load_source_manifest(&self, digest: &Digest) -> Manifest {
        self.app_context
            .cache_engine
            .storage
            .load_manifest(digest)
            .await
            .unwrap()
            .expect("manifest should have been seeded")
            .manifest
    }
}

impl Drop for TestDaemon {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
    }
}

fn action_digest() -> Digest {
    Digest::from_bytes(b"fingerprint").unwrap()
}

fn manifest_with_output(contents: &'static [u8]) -> Manifest {
    Manifest {
        files: vec![ManifestFile {
            bytes: Some(contents.into()),
            digest: Some(Digest::from_bytes(contents).unwrap()),
            path: "project/out.txt".into(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

mod archive {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_the_manifest_in_the_cache() {
        let daemon = TestDaemon::start().await;
        let action = action_digest();

        let response = daemon
            .client
            .clone()
            .archive_task_outputs(
                "app:build".into(),
                action.clone(),
                manifest_with_output(b"output"),
                true,
                false,
            )
            .await
            .unwrap();

        // The RPC queues the work and acks immediately.
        assert!(response.archived);
        assert!(daemon.wait_for_manifest(&action).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_the_output_blobs() {
        let daemon = TestDaemon::start().await;
        let action = action_digest();
        let output = Digest::from_bytes(b"output").unwrap();

        daemon
            .client
            .clone()
            .archive_task_outputs(
                "app:build".into(),
                action.clone(),
                manifest_with_output(b"output"),
                true,
                false,
            )
            .await
            .unwrap();

        assert!(daemon.wait_for_manifest(&action).await);
        assert!(
            daemon.blob_exists(&output).await,
            "the output blob must reach the CAS or hydration can't restore it"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn preserves_the_exit_code_and_console_output() {
        let daemon = TestDaemon::start().await;
        let action = action_digest();

        let mut manifest = manifest_with_output(b"output");
        manifest.exit_code = 3;
        manifest.stdout_bytes = Some(b"built ok".as_slice().into());
        manifest.stdout_digest = Some(Digest::from_bytes(b"built ok").unwrap());
        manifest.stderr_bytes = Some(b"warned".as_slice().into());
        manifest.stderr_digest = Some(Digest::from_bytes(b"warned").unwrap());

        daemon
            .client
            .clone()
            .archive_task_outputs("app:build".into(), action.clone(), manifest, true, false)
            .await
            .unwrap();

        assert!(daemon.wait_for_manifest(&action).await);

        let stored = daemon.load_source_manifest(&action).await;

        assert_eq!(stored.exit_code, 3);
        assert_eq!(
            stored.stdout_digest,
            Some(Digest::from_bytes(b"built ok").unwrap())
        );
        assert_eq!(
            stored.stderr_digest,
            Some(Digest::from_bytes(b"warned").unwrap())
        );
        // The console blobs have to be uploaded too, so a cache hit can replay
        // the terminal output rather than showing nothing.
        assert!(
            daemon
                .blob_exists(&Digest::from_bytes(b"built ok").unwrap())
                .await
        );
        assert!(
            daemon
                .blob_exists(&Digest::from_bytes(b"warned").unwrap())
                .await
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn drops_the_action_blob_over_the_rpc() {
        // KNOWN DEFECT, pinned here so a fix trips this test.
        //
        // `digest_source` names the fingerprint file that the action digest
        // addresses. `ActionResult` has no field for it, so the client's
        // `inherit_source` work is discarded in transit and the daemon never
        // uploads it. Backends that validate the RE contract then reject the
        // result with "action digest <hash>/<size> not found in CAS".
        //
        // When this is fixed, flip the daemon assertion to match the direct one.
        let daemon = TestDaemon::start().await;

        let mut manifest = manifest_with_output(b"output");
        manifest.digest_source = Some(ManifestFile {
            bytes: Some(b"fingerprint".as_slice().into()),
            digest: Some(action_digest()),
            path: ".moon/cache/hashes/abc.json".into(),
            ..Default::default()
        });

        // Archiving directly does upload it...
        let direct = Digest::from_bytes(b"direct").unwrap();
        daemon.seed(&direct, manifest.clone()).await;

        assert!(
            daemon.blob_exists(&action_digest()).await,
            "archiving in-process uploads the action blob"
        );

        // ...but going through the daemon loses it. Re-run against a cache with
        // no action blob present to observe it independently.
        let daemon = TestDaemon::start().await;
        let action = Digest::from_bytes(b"other-fingerprint").unwrap();

        manifest.digest_source = Some(ManifestFile {
            bytes: Some(b"other-fingerprint".as_slice().into()),
            digest: Some(action.clone()),
            path: ".moon/cache/hashes/def.json".into(),
            ..Default::default()
        });

        daemon
            .client
            .clone()
            .archive_task_outputs("app:build".into(), action.clone(), manifest, true, false)
            .await
            .unwrap();

        assert!(daemon.wait_for_manifest(&action).await);
        assert!(
            !daemon.blob_exists(&action).await,
            "action blob unexpectedly present — the RPC now carries digest_source, \
             so update this test to assert it IS uploaded"
        );
    }
}

mod hydrate {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn unpacks_outputs_into_the_workspace() {
        let daemon = TestDaemon::start().await;
        let action = action_digest();

        daemon.seed(&action, manifest_with_output(b"output")).await;

        let output_path = daemon.sandbox.path().join("project/out.txt");
        assert!(!output_path.exists());

        let response = daemon
            .client
            .clone()
            .hydrate_task_outputs(
                "app:build".into(),
                action,
                daemon.load_source_manifest(&action_digest()).await,
                true,
                false,
                LOCAL_BACKEND.into(),
            )
            .await
            .unwrap();

        assert!(response.hydrated);
        assert!(response.manifest.is_some());
        assert_eq!(std::fs::read_to_string(output_path).unwrap(), "output");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_console_output_but_strips_file_bytes() {
        // The client replays stdout/stderr on a cache hit, so those bytes have
        // to come back — but the file bytes were just written to disk, and
        // shipping them again would double the payload for nothing.
        let daemon = TestDaemon::start().await;
        let action = action_digest();

        let mut manifest = manifest_with_output(b"output");
        manifest.stdout_bytes = Some(b"built ok".as_slice().into());
        manifest.stdout_digest = Some(Digest::from_bytes(b"built ok").unwrap());

        daemon.seed(&action, manifest).await;

        let response = daemon
            .client
            .clone()
            .hydrate_task_outputs(
                "app:build".into(),
                action,
                daemon.load_source_manifest(&action_digest()).await,
                true,
                false,
                LOCAL_BACKEND.into(),
            )
            .await
            .unwrap();

        let result = response.manifest.unwrap();

        assert_eq!(result.stdout_raw, b"built ok");
        assert_eq!(result.output_files.len(), 1);
        assert!(
            result.output_files[0].contents.is_empty(),
            "file bytes should not be shipped back after being unpacked"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reports_a_miss_when_the_blobs_are_unavailable() {
        // Nothing was ever archived, so the referenced blob can't be resolved
        // and the task has to re-run. The manifest carries no inline bytes —
        // with them it would already be hydrated and never reach the CAS.
        let daemon = TestDaemon::start().await;

        let mut manifest = manifest_with_output(b"never-stored");
        manifest.files[0].bytes = None;

        let response = daemon
            .client
            .clone()
            .hydrate_task_outputs(
                "app:build".into(),
                action_digest(),
                manifest,
                true,
                false,
                LOCAL_BACKEND.into(),
            )
            .await
            .unwrap();

        // An absent blob surfaces as an error from the backend, which the
        // handler turns into a miss rather than failing the RPC — so the task
        // just re-runs. Nothing is left half-written in the workspace.
        assert!(!response.hydrated);
        assert!(response.manifest.is_none());
        assert!(!daemon.sandbox.path().join("project/out.txt").exists());

        // Note the sibling path is not this careful: when `hydrate_manifest`
        // returns `Ok(None)` (blobs simply absent from a remote, no error) the
        // handler still answers `hydrated: true` with no manifest. Nothing
        // reads the flag today, so it's latent rather than broken.
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejects_an_unknown_backend_id() {
        let daemon = TestDaemon::start().await;
        let action = action_digest();

        daemon.seed(&action, manifest_with_output(b"output")).await;

        let result = daemon
            .client
            .clone()
            .hydrate_task_outputs(
                "app:build".into(),
                action,
                daemon.load_source_manifest(&action_digest()).await,
                true,
                false,
                "does-not-exist".into(),
            )
            .await;

        // Note this is a hard error rather than a miss, so a backend-id
        // mismatch fails the task instead of just re-running it.
        assert!(result.is_err());
        assert!(!daemon.sandbox.path().join("project/out.txt").exists());
    }
}
