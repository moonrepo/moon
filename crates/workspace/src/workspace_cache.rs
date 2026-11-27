use crate::workspace_builder::WorkspaceBuilderContext;
use miette::IntoDiagnostic;
use moon_blob::Blob;
use moon_cache::{CacheEngine, ContentHash, cache_item};
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_common::{Id, is_docker};
use moon_env_var::GlobalEnvBag;
use moon_hash::{ContentHasher, Digest, fingerprint};
use moon_pdk_api::VirtualPath;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_utils::json::serde_json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::trace;

cache_item!(
    pub struct WorkspaceGraphCacheState {
        pub last_hash: ContentHash,

        /// The digest of the graph cached for `last_hash` in the actions
        /// storage, which is content-addressed by the serialized graph.
        pub graph_digest: Option<Digest>,

        /// Input files discovered by plugins while extending the graph
        /// during the last build. Since plugins only run on a cache miss,
        /// these must contribute to the next run's fingerprint.
        pub plugin_input_paths: BTreeSet<WorkspaceRelativePathBuf>,
    }
);

fingerprint!(
    #[derive(Debug)]
    pub struct WorkspaceGraphFingerprint<'graph> {
        // Project sources derived from the workspace graph builder.
        projects: BTreeMap<&'graph Id, &'graph WorkspaceRelativePathBuf>,

        // Environment variables required for cache invalidation.
        env: BTreeMap<String, String>,

        // Versions of the extension plugins that may extend the graph.
        extensions: BTreeMap<&'graph Id, &'graph String>,

        // The graph stores absolute file paths, which breaks moon when
        // running tasks inside and outside of a container at the same time.
        // This flag helps to continuously bust the cache.
        in_docker: bool,

        // Project and workspace configs, toolchain inputs, and `inheritedBy`
        // files required for cache invalidation.
        inputs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // Versions of the toolchain plugins that may extend the graph.
        toolchains: BTreeMap<&'graph Id, &'graph String>,

        // Version of the moon CLI. We need to include this so that the graph
        // cache is invalidated between each release, otherwise internal Rust
        // changes (in project or task crates) are not reflected until the cache
        // is invalidated, which puts the program in a weird state.
        version: String,

        // Absolute workspace root. The graph stores absolute project and task
        // paths, so a workspace that was moved or renamed cannot reuse a graph
        // built at its previous location.
        workspace_root: PathBuf,
    }
);

impl Default for WorkspaceGraphFingerprint<'_> {
    fn default() -> Self {
        WorkspaceGraphFingerprint {
            projects: BTreeMap::default(),
            inputs: BTreeMap::default(),
            env: BTreeMap::default(),
            in_docker: is_docker(),
            extensions: BTreeMap::default(),
            toolchains: BTreeMap::default(),
            version: GlobalEnvBag::instance()
                .get("MOON_VERSION")
                .unwrap_or_default(),
            workspace_root: PathBuf::default(),
        }
    }
}

impl<'graph> WorkspaceGraphFingerprint<'graph> {
    pub fn add_projects(&mut self, projects: &'graph BTreeMap<Id, WorkspaceRelativePathBuf>) {
        self.projects.extend(projects.iter());
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

    pub fn set_workspace_root(&mut self, root: &Path) {
        self.workspace_root = root.to_path_buf();
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

/// Map plugin provided input files (absolute paths on the host machine)
/// into workspace relative paths for use within the graph fingerprint.
/// Files outside of the workspace root cannot be hashed, so are skipped.
pub fn map_plugin_input_paths(
    workspace_root: &Path,
    input_files: Vec<VirtualPath>,
    paths: &mut BTreeSet<WorkspaceRelativePathBuf>,
) {
    for file in input_files {
        if file.starts_with(workspace_root)
            && let Ok(rel_file) = file.as_path().relative_to(workspace_root)
        {
            paths.insert(rel_file);
        } else {
            trace!(
                file = ?file,
                "Skipping plugin input file outside of the workspace root",
            );
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

    context
        .cache_engine
        .hash_files(&context.workspace_root, &paths)
        .await
}

/// Tasks may only be inherited when a file exists within a project
/// (`inheritedBy.files`), so these files must be hashed, otherwise adding,
/// removing, or changing them would not invalidate the cache. Every possible
/// path is returned, as missing files are omitted when hashing.
fn get_inherited_by_paths(
    context: &WorkspaceBuilderContext,
    projects: &BTreeMap<Id, WorkspaceRelativePathBuf>,
) -> BTreeSet<WorkspaceRelativePathBuf> {
    let file_names = context
        .inherited_tasks
        .configs
        .iter()
        .filter_map(|entry| entry.config.inherited_by.as_ref()?.files.as_ref())
        .flat_map(|files| files.to_list())
        .map(|file| file.as_str())
        .collect::<BTreeSet<_>>();

    let mut paths = BTreeSet::default();

    for file_name in file_names {
        for source in projects.values() {
            paths.insert(source.join(file_name));
        }
    }

    paths
}

/// Create a hasher for the current workspace, derived from project
/// sources, config and `inheritedBy` file contents, plugin input files
/// (discovered while extending the graph during the previous build),
/// plugin versions, and environment variables. Its hash is used to
/// invalidate the cached workspace graph. The hasher is not stored as
/// a manifest, as the hash may be regenerated before it's stored.
pub async fn create_graph_cache_hasher(
    context: Arc<WorkspaceBuilderContext>,
    projects: &BTreeMap<Id, WorkspaceRelativePathBuf>,
    config_paths: BTreeSet<WorkspaceRelativePathBuf>,
    plugin_input_paths: BTreeSet<WorkspaceRelativePathBuf>,
) -> miette::Result<ContentHasher> {
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

    let project_sources = projects
        .values()
        .map(|source| source.to_string())
        .collect::<Vec<_>>();

    let toolchain_context = Arc::clone(&context);
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
    all_paths.extend(plugin_input_paths);
    all_paths.extend(get_inherited_by_paths(&context, projects));

    let mut fingerprint = WorkspaceGraphFingerprint::default();
    fingerprint.add_projects(projects);
    fingerprint.add_inputs(hash_input_paths(&context, all_paths).await?);
    fingerprint.add_extension_versions(&extension_versions);
    fingerprint.add_toolchain_versions(&toolchain_versions);
    fingerprint.gather_env();
    fingerprint.set_workspace_root(&context.workspace_root);

    let mut hasher = ContentHasher::new("workspace-graph");
    hasher.hash_content(&fingerprint)?;

    Ok(hasher)
}

/// Read the cached graph with the given digest from the actions storage, if
/// it exists. The blob is written by moon, and never contains comments, so
/// it's parsed directly from bytes, without stripping comments, or converting
/// to a string.
pub async fn read_cache_blob<T: DeserializeOwned>(
    cache_engine: &CacheEngine,
    digest: &Digest,
) -> miette::Result<Option<T>> {
    let Some(blob) = cache_engine
        .actions_storage
        .retrieve_blob(digest.to_owned())
        .await?
    else {
        return Ok(None);
    };

    // No task manifest references the graph, so mark it as used,
    // otherwise it would be cleaned once older than the lifetime,
    // even while it's still the current graph
    cache_engine
        .actions_storage
        .touch_blobs(vec![digest.to_owned()])
        .await;

    let data = blob.content.read_bytes()?;

    serde_json::from_slice(&data).map(Some).into_diagnostic()
}

/// Write the cached graph to the actions storage, serialized directly into
/// bytes instead of a string, and return its digest. Blobs are written
/// atomically, so a partially written graph is never read.
pub async fn write_cache_blob<T: Serialize>(
    cache_engine: &CacheEngine,
    data: &T,
) -> miette::Result<Digest> {
    let blob = Blob::from_bytes(serde_json::to_vec(data).into_diagnostic()?)?;
    let digest = blob.digest.clone();

    // Replace rather than store, so that a graph that couldn't be read back,
    // and was rebuilt with the same content (and digest), is repaired
    cache_engine
        .actions_storage
        .replace_blobs(vec![blob.into_input()])
        .await?;

    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_hash::{ContentHasher, Digest};

    fn digest_for_root(root: &str) -> String {
        let mut fingerprint = WorkspaceGraphFingerprint::default();
        fingerprint.set_workspace_root(Path::new(root));

        let mut hasher = ContentHasher::new("workspace-graph");
        hasher.hash_content(&fingerprint).unwrap();

        Digest::from_hasher(&mut hasher).unwrap().hash.to_string()
    }

    #[test]
    fn changing_the_workspace_root_changes_the_graph_digest() {
        assert_eq!(
            digest_for_root("/repo/workspace"),
            digest_for_root("/repo/workspace")
        );
        assert_ne!(
            digest_for_root("/repo/workspace"),
            digest_for_root("/repo/moved-workspace")
        );
    }
}
