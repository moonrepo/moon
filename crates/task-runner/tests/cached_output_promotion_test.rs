mod utils;

use moon_blob::{BlobContent, BlobInput, BlobOutput};
use moon_cache::{
    CacheCapabilities, CacheContext, Digest, StorageBackend, StorageOptions, TaskManifest,
    TaskManifestSource,
};
use moon_cache_local::LocalStorage;
use moon_common::Id;
use moon_task_runner::TaskRunner;
use moon_task_runner::output_archiver::ArchiveOutcome;
use starbase_archive::Archiver;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use utils::TaskRunnerContainer;

#[derive(Debug)]
struct ProbeBackend {
    id: Id,
    capabilities: CacheCapabilities,
    connected: Arc<AtomicBool>,
    lookups: Arc<AtomicUsize>,
    writable: bool,
    stall_lookup: bool,
}

#[async_trait::async_trait]
impl StorageBackend for ProbeBackend {
    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_capabilities(&self) -> &CacheCapabilities {
        &self.capabilities
    }

    fn is_readable(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn is_writable(&self) -> bool {
        self.writable && self.connected.load(Ordering::SeqCst)
    }

    async fn connect(&self) -> miette::Result<()> {
        tokio::task::yield_now().await;
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn retrieve_task_manifest(
        &self,
        _digest: Digest,
    ) -> miette::Result<Option<TaskManifest>> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        if self.stall_lookup {
            std::future::pending::<()>().await;
        }
        Err(miette::miette!("Unexpected promotion manifest lookup"))
    }

    async fn store_task_manifest(
        &self,
        _digest: Digest,
        _manifest: TaskManifest,
    ) -> miette::Result<()> {
        panic!("Read-only backend must not receive writes")
    }

    async fn find_missing_blobs(&self, _digests: Vec<Digest>) -> miette::Result<Vec<Digest>> {
        panic!("Read-only promotion must not look up blobs")
    }

    async fn store_blobs(
        &self,
        _blobs: Vec<BlobInput>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        panic!("Read-only backend must not receive writes")
    }

    async fn retrieve_blobs(
        &self,
        _digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<BlobOutput>> {
        panic!("Read-only promotion must not retrieve blobs")
    }
}

fn add_remote_storage(container: &mut TaskRunnerContainer, cache_dir: &Path) {
    let workspace = &container.app_context.workspace_config;
    let mut cache_config = workspace.cache.clone();
    cache_config.shared_worktree_cache = true;

    let context = CacheContext {
        cache_dir: cache_dir.to_path_buf(),
        cache_shared_dir: Some(cache_dir.to_path_buf()),
        cache_config: Arc::new(cache_config),
        config_dir: cache_dir.to_path_buf(),
        remote_config: Arc::new(workspace.remote.clone()),
        remote_debug: false,
        workspace_root: container.sandbox.path().to_path_buf(),
    };
    let storage = LocalStorage::new(context.clone(), cache_dir).unwrap();
    let app_context =
        Arc::get_mut(&mut container.app_context).expect("test container owns its app context");
    let cache_engine =
        Arc::get_mut(&mut app_context.cache_engine).expect("test container owns its cache engine");
    cache_engine.storage.add_remote_backend(storage);
}

async fn save_fingerprint(container: &TaskRunnerContainer) -> Digest {
    container
        .app_context
        .cache_engine
        .storage
        .store_hash_manifest("cached-output-promotion", "fingerprint")
        .await
        .unwrap()
}

fn pack_legacy_archive(container: &TaskRunnerContainer, hash: &str, output: &str) {
    pack_archive_snapshot(
        container,
        hash,
        &[
            ("project/file.txt", output),
            ("project/undeclared.txt", "must not be published"),
        ],
    );
}

fn pack_archive_snapshot(container: &TaskRunnerContainer, hash: &str, outputs: &[(&str, &str)]) {
    let root = container.sandbox.path();
    let stdout_path = format!(
        ".moon/cache/states/{}/{}/stdout.log",
        container.project_id, container.task_id
    );
    let stderr_path = format!(
        ".moon/cache/states/{}/{}/stderr.log",
        container.project_id, container.task_id
    );
    container
        .sandbox
        .create_file(&stdout_path, "archived stdout");
    container
        .sandbox
        .create_file(&stderr_path, "archived stderr");

    for (path, contents) in outputs {
        container.sandbox.create_file(path, contents);
    }

    let archive_path = container
        .app_context
        .cache_engine
        .hash
        .get_archive_path(hash);
    let mut archiver = Archiver::new(root, &archive_path);
    for (path, _) in outputs {
        archiver.add_source_file(path, None);
    }
    archiver.add_source_file(&stdout_path, None);
    archiver.add_source_file(&stderr_path, None);
    archiver.pack_from_ext().unwrap();
}

async fn remote_manifest(
    container: &TaskRunnerContainer,
    digest: &Digest,
) -> Option<moon_cache::TaskManifest> {
    container
        .app_context
        .cache_engine
        .storage
        .with_options(StorageOptions {
            include_local: false,
            include_remote: true,
            ..Default::default()
        })
        .load_task_manifest(digest)
        .await
        .unwrap()
        .map(|source| source.manifest)
}

async fn hydrate_remote_manifest(
    container: &TaskRunnerContainer,
    digest: &Digest,
) -> moon_cache::TaskManifest {
    let remote = container
        .app_context
        .cache_engine
        .storage
        .with_options(StorageOptions {
            include_local: false,
            include_remote: true,
            ..Default::default()
        });
    let source: TaskManifestSource = remote
        .load_task_manifest(digest)
        .await
        .unwrap()
        .expect("remote action cache entry exists");
    remote
        .hydrate_task_manifest(digest, source)
        .await
        .unwrap()
        .expect("remote output blobs exist")
}

fn read_manifest_file(file: &moon_cache::TaskManifestFile) -> Vec<u8> {
    if let Some(bytes) = &file.bytes {
        bytes.to_vec()
    } else {
        fs::read(
            file.source_path
                .as_ref()
                .expect("file bytes or source path"),
        )
        .unwrap()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn promotes_saved_outputs_and_archived_logs_after_previous_output_hit() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);

    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&container, hash, "saved output");
    container
        .sandbox
        .create_file("project/file.txt", "tampered live output");
    container.sandbox.create_file(
        ".moon/cache/states/project/file-outputs/stdout.log",
        "live stdout",
    );
    container.sandbox.create_file(
        ".moon/cache/states/project/file-outputs/stderr.log",
        "live stderr",
    );

    let mut runner = container.create_runner();
    runner.cache.data.exit_code = 0;
    runner.cache.data.hash = hash.to_owned();
    runner.state.digest = digest.clone();
    runner.state.remote_cache_writable = true;

    assert!(
        runner.hydrate(hash).await.unwrap(),
        "the local hit remains successful"
    );
    container.flush_storage().await;

    let manifest = hydrate_remote_manifest(&container, &digest).await;
    let output = manifest
        .files
        .iter()
        .find(|file| file.path.as_str() == "project/file.txt")
        .expect("declared output is promoted");
    assert_eq!(read_manifest_file(output), b"saved output");
    assert!(
        manifest
            .files
            .iter()
            .all(|file| file.path.as_str() != "project/undeclared.txt"),
        "undeclared tar entries are not promoted"
    );
    assert_eq!(
        manifest.stdout_bytes.as_ref().unwrap().as_ref(),
        b"archived stdout"
    );
    assert_eq!(
        manifest.stderr_bytes.as_ref().unwrap().as_ref(),
        b"archived stderr"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_workspace_restores_promoted_entry_from_remote_only() {
    let mut producer = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = producer.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut producer, &remote_dir);

    let digest = save_fingerprint(&producer).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&producer, hash, "published output");
    let mut state = producer.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;
    assert!(matches!(
        producer
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    producer.flush_storage().await;

    let mut consumer = TaskRunnerContainer::new("archive", "file-outputs").await;
    add_remote_storage(&mut consumer, &remote_dir);
    let mut runner = consumer.create_runner();
    runner.state.digest = digest.clone();

    assert!(
        runner.hydrate(hash).await.unwrap(),
        "remote cache hit hydrates"
    );
    assert_eq!(
        fs::read_to_string(consumer.sandbox.path().join("project/file.txt")).unwrap(),
        "published output"
    );
    assert!(
        remote_manifest(&consumer, &digest).await.is_some(),
        "the consumer reads the published remote action cache entry"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn promotes_manifest_and_blobs_from_local_cas() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);

    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    container
        .sandbox
        .create_file("project/file.txt", "saved CAS output");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.local_cas_enabled = true;
    state.remote_cache_writable = false;

    assert!(matches!(
        container
            .create_archiver()
            .archive(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;
    assert!(remote_manifest(&container, &digest).await.is_none());

    container
        .sandbox
        .create_file("project/file.txt", "tampered live output");
    state.remote_cache_writable = true;
    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;

    let manifest = hydrate_remote_manifest(&container, &digest).await;
    let output = manifest
        .files
        .iter()
        .find(|file| file.path.as_str() == "project/file.txt")
        .expect("declared output is promoted");
    assert_eq!(read_manifest_file(output), b"saved CAS output");
}

#[tokio::test(flavor = "multi_thread")]
async fn promotes_required_glob_matches_from_the_saved_snapshot() {
    let mut container = TaskRunnerContainer::new("archive", "glob-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_archive_snapshot(
        &container,
        hash,
        &[
            ("project/one.txt", "saved one"),
            ("project/two.txt", "saved two"),
        ],
    );
    container
        .sandbox
        .create_file("project/one.txt", "changed one");
    container
        .sandbox
        .create_file("project/two.txt", "changed two");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;
    let manifest = hydrate_remote_manifest(&container, &digest).await;
    for (path, expected) in [
        ("project/one.txt", b"saved one".as_slice()),
        ("project/two.txt", b"saved two".as_slice()),
    ] {
        let file = manifest
            .files
            .iter()
            .find(|file| file.path.as_str() == path)
            .expect("glob match is promoted");
        assert_eq!(read_manifest_file(file), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn skips_required_glob_promotion_when_snapshot_has_no_match() {
    let mut container = TaskRunnerContainer::new("archive", "glob-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_archive_snapshot(
        &container,
        hash,
        &[("project/not-output.bin", "not a glob match")],
    );
    // A live match must not make an archive with no matching entry promotable.
    container
        .sandbox
        .create_file("project/live.txt", "current output");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    container.flush_storage().await;
    assert!(remote_manifest(&container, &digest).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn promotes_non_empty_declared_directory_from_the_saved_snapshot() {
    let mut container = TaskRunnerContainer::new("archive", "output-one-dir").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_archive_snapshot(
        &container,
        hash,
        &[("project/dir/file.txt", "saved directory output")],
    );
    container
        .sandbox
        .create_file("project/dir/file.txt", "changed directory output");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;
    let manifest = hydrate_remote_manifest(&container, &digest).await;
    let output = manifest
        .files
        .iter()
        .find(|file| file.path.as_str() == "project/dir/file.txt")
        .expect("directory child is promoted");
    assert_eq!(read_manifest_file(output), b"saved directory output");
}

#[tokio::test(flavor = "multi_thread")]
async fn permits_promotion_when_optional_output_is_absent() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs-optional").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_archive_snapshot(&container, hash, &[]);
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;
    let manifest = hydrate_remote_manifest(&container, &digest).await;
    assert!(manifest.files.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn promotes_workspace_root_outputs_from_the_saved_snapshot() {
    let mut container = TaskRunnerContainer::new("archive", "output-workspace").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_archive_snapshot(
        &container,
        hash,
        &[
            ("root.txt", "saved root output"),
            ("shared/a.txt", "saved shared output"),
        ],
    );
    container
        .sandbox
        .create_file("root.txt", "changed root output");
    container
        .sandbox
        .create_file("shared/a.txt", "changed shared output");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Queued
    ));
    container.flush_storage().await;
    let manifest = hydrate_remote_manifest(&container, &digest).await;
    for (path, expected) in [
        ("root.txt", b"saved root output".as_slice()),
        ("shared/a.txt", b"saved shared output".as_slice()),
    ] {
        let file = manifest
            .files
            .iter()
            .find(|file| file.path.as_str() == path)
            .expect("workspace-root output is promoted");
        assert_eq!(read_manifest_file(file), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn skips_promotion_when_remote_cache_is_read_only() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&container, hash, "saved output");

    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = false;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    container.flush_storage().await;
    assert!(remote_manifest(&container, &digest).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn connected_read_only_backend_skips_promotion_without_manifest_lookup() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&container, hash, "saved output");
    let connected = Arc::new(AtomicBool::new(false));
    let lookups = Arc::new(AtomicUsize::new(0));
    let app_context = Arc::get_mut(&mut container.app_context).unwrap();
    let cache_engine = Arc::get_mut(&mut app_context.cache_engine).unwrap();
    cache_engine.storage.add_remote_backend(ProbeBackend {
        id: Id::raw("read-only"),
        capabilities: CacheCapabilities::default(),
        connected: Arc::clone(&connected),
        lookups: Arc::clone(&lookups),
        writable: false,
        stall_lookup: false,
    });
    cache_engine.storage.spawn_connect_backends();
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    assert!(connected.load(Ordering::SeqCst));
    assert_eq!(lookups.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn hung_remote_lookup_stays_within_promotion_budget() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.to_string();
    pack_legacy_archive(&container, &hash, "saved output");
    let connected = Arc::new(AtomicBool::new(false));
    let lookups = Arc::new(AtomicUsize::new(0));
    let app_context = Arc::get_mut(&mut container.app_context).unwrap();
    let cache_engine = Arc::get_mut(&mut app_context.cache_engine).unwrap();
    cache_engine.storage.add_remote_backend(ProbeBackend {
        id: Id::raw("hung-lookup"),
        capabilities: CacheCapabilities::default(),
        connected: Arc::clone(&connected),
        lookups: Arc::clone(&lookups),
        writable: true,
        stall_lookup: true,
    });
    cache_engine.storage.spawn_connect_backends();
    let mut state = container.create_state();
    state.digest = digest;
    state.remote_cache_writable = true;

    let promotion = tokio::spawn(async move {
        let result = container
            .create_archiver()
            .promote_cached(&hash, &state)
            .await;
        let still_enabled = container
            .app_context
            .cache_engine
            .storage
            .is_promotion_enabled();
        (result, still_enabled)
    });
    for _ in 0..1_000 {
        if lookups.load(Ordering::SeqCst) != 0 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(lookups.load(Ordering::SeqCst), 1);
    tokio::time::advance(moon_cache::PROMOTION_TIMEOUT + std::time::Duration::from_secs(1)).await;
    let (result, still_enabled) = promotion.await.unwrap();
    let Err(error) = result else {
        panic!("Hung manifest lookup unexpectedly promoted cached outputs");
    };
    assert!(
        error
            .to_string()
            .contains("Timed out promoting cached outputs")
    );
    assert!(!still_enabled);
}

#[tokio::test(flavor = "multi_thread")]
async fn skips_promotion_when_fingerprint_file_is_missing_or_stale() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&container, hash, "saved output");
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    let fingerprint_content = container
        .app_context
        .cache_engine
        .storage
        .load_hash_manifest(&digest)
        .await
        .unwrap()
        .unwrap();
    let BlobContent::File(fingerprint_path) = fingerprint_content else {
        panic!("local hash manifest should be file-backed");
    };
    fs::remove_file(&fingerprint_path).unwrap();
    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));

    fs::write(&fingerprint_path, "stale fingerprint").unwrap();
    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(&format!("{hash}-stale"), &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    assert!(remote_manifest(&container, &digest).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn skips_promotion_when_archive_lacks_a_required_output() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    container.sandbox.create_file(
        ".moon/cache/states/project/file-outputs/stdout.log",
        "only log",
    );
    container
        .sandbox
        .create_file("project/file.txt", "valid live output");
    let archive_path = container
        .app_context
        .cache_engine
        .hash
        .get_archive_path(hash);
    let mut archiver = Archiver::new(container.sandbox.path(), &archive_path);
    archiver.add_source_file(".moon/cache/states/project/file-outputs/stdout.log", None);
    archiver.pack_from_ext().unwrap();
    let mut state = container.create_state();
    state.digest = digest.clone();
    state.remote_cache_writable = true;

    assert!(matches!(
        container
            .create_archiver()
            .promote_cached(hash, &state)
            .await
            .unwrap(),
        ArchiveOutcome::Skipped
    ));
    container.flush_storage().await;
    assert!(remote_manifest(&container, &digest).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn cached_failed_exit_code_minus_one_is_not_admitted() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    container.sandbox.enable_git();
    container
        .sandbox
        .create_file("project/file.txt", "existing output");
    pack_legacy_archive(&container, hash, "failed output");

    let mut runner = container.create_runner();
    runner.state.digest = digest.clone();
    runner.cache.data.exit_code = -1;
    runner.cache.data.hash = hash.to_owned();

    assert!(!runner.hydrate(hash).await.unwrap());
    assert!(remote_manifest(&container, &digest).await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_or_corrupt_archive_does_not_invalidate_previous_output_hit() {
    for corrupt in [false, true] {
        let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
        let remote_dir = container.sandbox.path().join("isolated-remote-cache");
        add_remote_storage(&mut container, &remote_dir);
        let digest = save_fingerprint(&container).await;
        let hash = digest.hash.as_str();
        container.sandbox.enable_git();
        container
            .sandbox
            .create_file("project/file.txt", "still valid cached output");
        if corrupt {
            container.sandbox.create_file(
                format!(".moon/cache/outputs/{hash}.tar.gz"),
                "not a tar archive",
            );
        }

        let mut runner = container.create_runner();
        runner.state.digest = digest.clone();
        runner.state.remote_cache_writable = true;
        runner.cache.data.exit_code = 0;
        runner.cache.data.hash = hash.to_owned();

        assert!(
            runner.hydrate(hash).await.unwrap(),
            "local cache hit must survive absent or corrupt promotion source"
        );
        assert_eq!(
            fs::read_to_string(container.sandbox.path().join("project/file.txt")).unwrap(),
            "still valid cached output"
        );
        assert!(remote_manifest(&container, &digest).await.is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn persistent_previous_output_hit_is_not_promoted() {
    let mut container = TaskRunnerContainer::new("archive", "file-outputs").await;
    let remote_dir = container.sandbox.path().join("isolated-remote-cache");
    add_remote_storage(&mut container, &remote_dir);
    let digest = save_fingerprint(&container).await;
    let hash = digest.hash.as_str();
    pack_legacy_archive(&container, hash, "saved output");
    container
        .sandbox
        .create_file("project/file.txt", "existing cached output");

    let mut task = container.task.as_ref().clone();
    task.options.persistent = true;
    let task = Arc::new(task);
    let mut runner =
        TaskRunner::new(&container.app_context, &container.project, &task, None).unwrap();
    runner.cache.data.exit_code = 0;
    runner.cache.data.hash = hash.to_owned();
    runner.state.digest = digest.clone();
    runner.state.remote_cache_writable = true;

    assert!(
        runner.hydrate(hash).await.unwrap(),
        "persistent task cache lookup remains a successful hit"
    );
    assert_eq!(
        fs::read_to_string(container.sandbox.path().join("project/file.txt")).unwrap(),
        "existing cached output"
    );
    assert!(remote_manifest(&container, &digest).await.is_none());
}
