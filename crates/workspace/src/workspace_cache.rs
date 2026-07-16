use crate::projects_builder::{ProjectBuildData, ProjectBuildDataMap};
use crate::workspace_builder::WorkspaceBuilderContext;
use miette::IntoDiagnostic;
use moon_cache::{ContentHash, cache_item};
use moon_cache_storage::{Manifest, ManifestFile};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, is_docker};
use moon_env_var::GlobalEnvBag;
use moon_hash::{Digest, fingerprint};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

cache_item!(
    pub struct WorkspaceGraphCacheState {
        pub last_hash: ContentHash,
    }
);

fingerprint!(
    #[derive(Debug)]
    pub struct WorkspaceGraphFingerprint<'graph> {
        // Project sources derived from the workspace graph builder.
        projects: BTreeMap<&'graph Id, &'graph WorkspaceRelativePathBuf>,

        // Whether the graph was built with the async builder. The builders
        // serialize into different shapes, so they cannot share a hash.
        async_graph_building: bool,

        // Environment variables required for cache invalidation.
        env: BTreeMap<String, String>,

        // Versions of the extension plugins that may extend the graph.
        extensions: BTreeMap<&'graph Id, &'graph String>,

        // The graph stores absolute file paths, which breaks moon when
        // running tasks inside and outside of a container at the same time.
        // This flag helps to continuously bust the cache.
        in_docker: bool,

        // Project and workspace configs and toolchain inputs required
        // for cache invalidation.
        inputs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // Versions of the toolchain plugins that may extend the graph.
        toolchains: BTreeMap<&'graph Id, &'graph String>,

        // Version of the moon CLI. We need to include this so that the graph
        // cache is invalidated between each release, otherwise internal Rust
        // changes (in project or task crates) are not reflected until the cache
        // is invalidated, which puts the program in a weird state.
        version: String,
    }
);

impl Default for WorkspaceGraphFingerprint<'_> {
    fn default() -> Self {
        WorkspaceGraphFingerprint {
            projects: BTreeMap::default(),
            async_graph_building: false,
            inputs: BTreeMap::default(),
            env: BTreeMap::default(),
            in_docker: is_docker(),
            extensions: BTreeMap::default(),
            toolchains: BTreeMap::default(),
            version: GlobalEnvBag::instance()
                .get("MOON_VERSION")
                .unwrap_or_default(),
        }
    }
}

impl<'graph> WorkspaceGraphFingerprint<'graph> {
    pub fn set_async_graph_building(&mut self, value: bool) {
        self.async_graph_building = value;
    }

    pub fn add_projects(&mut self, projects: &'graph FxHashMap<Id, ProjectBuildData>) {
        self.projects.extend(
            projects
                .iter()
                .map(|(id, build_data)| (id, &build_data.source)),
        );
    }

    pub fn add_inputs(&mut self, inputs: BTreeMap<WorkspaceRelativePathBuf, String>) {
        self.inputs.extend(inputs);
    }

    pub fn add_extension_versions(&mut self, versions: &'graph BTreeMap<Id, String>) {
        self.extensions.extend(versions.iter());
    }

    pub fn add_toolchain_versions(&mut self, versions: &'graph BTreeMap<Id, String>) {
        self.toolchains.extend(versions.iter());
    }

    pub fn gather_env(&mut self) {
        let bag = GlobalEnvBag::instance();

        for key in [
            // Task options
            "MOON_OUTPUT_STYLE",
            "MOON_RETRY_COUNT",
        ] {
            self.env
                .insert(key.to_owned(), bag.get(key).unwrap_or_default());
        }
    }
}

/// When hashing the graph, we must hash all project and workspace
/// config files, and possible plugin input files, that are required
/// to invalidate the cache. Missing files are simply omitted from
/// the result, so that file existence contributes to the hash.
async fn hash_input_paths(
    context: &WorkspaceBuilderContext,
    paths: BTreeSet<WorkspaceRelativePathBuf>,
) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
    let paths = paths.into_iter().collect::<Vec<_>>();

    if context.workspace_config.experiments.native_file_hashing {
        context
            .cache_engine
            .hash_files(&context.workspace_root, &paths)
            .await
    } else {
        context
            .vcs
            .as_ref()
            .expect("VCS required!")
            .get_file_hashes(&paths, true)
            .await
    }
}

/// Generate a digest for the current workspace, derived from project
/// sources, config file contents, plugin input files, plugin versions,
/// and environment variables. This digest is used to invalidate the
/// cached workspace graph.
pub async fn generate_graph_cache_digest(
    context: Arc<WorkspaceBuilderContext>,
    project_data: &ProjectBuildDataMap,
    config_paths: BTreeSet<WorkspaceRelativePathBuf>,
    async_graph_building: bool,
) -> miette::Result<Digest> {
    let extension_context = Arc::clone(&context);
    let extension_handle = tokio::spawn(async move {
        let mut versions = BTreeMap::default();

        for extension in extension_context.extension_registry.load_all().await? {
            if extension.has_func("extend_project_graph").await {
                versions.insert(
                    extension.id.clone(),
                    extension.metadata.plugin_version.clone(),
                );
            }
        }

        Ok::<_, miette::Report>(versions)
    });

    let toolchain_context = Arc::clone(&context);
    let project_sources = project_data
        .values()
        .map(|build_data| build_data.source.to_string())
        .collect::<Vec<_>>();
    let toolchain_handle = tokio::spawn(async move {
        let mut paths = BTreeSet::default();
        let mut versions = BTreeMap::default();

        for toolchain in toolchain_context.toolchain_registry.load_all().await? {
            for file_name in &toolchain.metadata.manifest_file_names {
                // In the workspace root, which may not be a project
                paths.insert(WorkspaceRelativePathBuf::from(file_name.as_str()));

                // And in each project source directory
                for source in &project_sources {
                    paths.insert(WorkspaceRelativePathBuf::from(source).join(file_name));
                }
            }

            if toolchain.has_func("extend_project_graph").await {
                versions.insert(
                    toolchain.id.clone(),
                    toolchain.metadata.plugin_version.clone(),
                );
            }
        }

        Ok::<_, miette::Report>((paths, versions))
    });

    let extension_versions = extension_handle.await.into_diagnostic()??;
    let (toolchain_paths, toolchain_versions) = toolchain_handle.await.into_diagnostic()??;

    let mut all_paths = config_paths;
    all_paths.extend(toolchain_paths);

    let mut fingerprint = WorkspaceGraphFingerprint::default();
    fingerprint.set_async_graph_building(async_graph_building);
    fingerprint.add_projects(project_data);
    fingerprint.add_inputs(hash_input_paths(&context, all_paths).await?);
    fingerprint.add_extension_versions(&extension_versions);
    fingerprint.add_toolchain_versions(&toolchain_versions);
    fingerprint.gather_env();

    context
        .cache_engine
        .hash
        .save_manifest_without_hasher("workspace-graph", &fingerprint)
}

/// Check whether the serialized workspace graph should also be persisted
/// to storage backends (typically remote), and if so, connect the backends.
/// The graph is built before the pipeline's sync workspace action, which
/// normally connects the backends, so we must connect here. Connecting is
/// idempotent, and failures disable the offending backend.
pub async fn is_remote_graph_cache_enabled(context: &WorkspaceBuilderContext) -> bool {
    if !context
        .workspace_config
        .experiments
        .remote_workspace_graph_cache
        || !context.cache_engine.storage.is_remote_enabled()
    {
        return false;
    }

    if let Err(error) = context.cache_engine.storage.connect_backends().await {
        warn!("Failed to connect to storage backends for workspace graph caching: {error}");

        return false;
    }

    true
}

/// Attempt to hydrate the serialized workspace graph from storage backends
/// (typically remote) when the local state is missing or stale. The graph
/// is keyed by the same fingerprint digest as the local state, so a fetched
/// entry always matches the current workspace inputs. Failures are lossy
/// and non-fatal, as this is purely an optimization over rebuilding.
pub async fn load_graph_from_storage(
    context: &WorkspaceBuilderContext,
    digest: &Digest,
    cache_path: &Path,
) -> bool {
    let storage = &context.cache_engine.storage;

    let result: miette::Result<bool> = async {
        let Some(source) = storage.load_manifest(digest).await? else {
            return Ok(false);
        };

        let Some(manifest) = storage.hydrate_manifest(digest, source).await? else {
            return Ok(false);
        };

        let Some(file) = manifest.files.first() else {
            return Ok(false);
        };

        if let Some(bytes) = &file.bytes {
            fs::write_file(cache_path, bytes)?;
        } else if let Some(source_path) = &file.source_path {
            fs::copy_file(source_path, cache_path)?;
        } else {
            return Ok(false);
        }

        Ok(true)
    }
    .await;

    match result {
        Ok(hydrated) => {
            if hydrated {
                debug!(
                    hash = digest.hash.as_str(),
                    "Hydrated workspace graph from storage backends"
                );
            }

            hydrated
        }
        Err(error) => {
            warn!(
                hash = digest.hash.as_str(),
                "Failed to hydrate workspace graph from storage backends: {error}"
            );

            false
        }
    }
}

/// Persist the serialized workspace graph to storage backends (typically
/// remote), keyed by its fingerprint digest, so that other machines can
/// hydrate it instead of rebuilding the graph from scratch. The upload is
/// awaited so that short-lived commands don't drop it. Failures are non-fatal.
pub async fn save_graph_to_storage(
    context: &WorkspaceBuilderContext,
    digest: &Digest,
    cache_path: &Path,
) {
    let result: miette::Result<()> = async {
        // The cache directory can technically be relocated outside of the
        // workspace, in which case a workspace relative path can not be
        // created, so simply don't persist the graph.
        let Ok(relative_path) = cache_path.strip_prefix(&context.workspace_root) else {
            return Ok(());
        };

        let Some(relative_path) = relative_path.to_str() else {
            return Ok(());
        };

        let manifest = Manifest {
            files: vec![ManifestFile {
                digest: Some(Digest::from_file(cache_path)?),
                path: WorkspaceRelativePathBuf::from(relative_path),
                source_path: Some(cache_path.to_path_buf()),
                ..Default::default()
            }],
            ..Default::default()
        };

        context
            .cache_engine
            .storage
            .archive_manifest(digest, manifest)
            .await?;

        // Archiving queues the upload as a background task, which is only
        // awaited when a task pipeline runs, so await it here for
        // graph-only commands (query, docker, etc.)
        context
            .cache_engine
            .storage
            .wait_for_background_tasks()
            .await
    }
    .await;

    if let Err(error) = result {
        warn!(
            hash = digest.hash.as_str(),
            "Failed to persist workspace graph to storage backends: {error}"
        );
    }
}
