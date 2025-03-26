use crate::build_data::ProjectBuildData;
use moon_cache::cache_item;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{Id, is_docker};
use moon_env_var::GlobalEnvBag;
use moon_hash::hash_content;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

cache_item!(
    pub struct WorkspaceProjectsCacheState {
        pub last_hash: String,
        pub projects: FxHashMap<Id, ProjectBuildData>,
    }
);

hash_content!(
    pub struct WorkspaceGraphHash<'graph> {
        // Data derived from the workspace graph builder.
        projects: BTreeMap<&'graph Id, &'graph ProjectBuildData>,

        // Project and workspace configs required for cache invalidation.
        configs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // Environment variables required for cache invalidation.
        env: BTreeMap<String, String>,

        // The graph stores absolute file paths, which breaks moon when
        // running tasks inside and outside of a container at the same time.
        // This flag helps to continuously bust the cache.
        in_docker: bool,

        // Version of the moon CLI. We need to include this so that the graph
        // cache is invalidated between each release, otherwise internal Rust
        // changes (in project or task crates) are not reflected until the cache
        // is invalidated, which puts the program in a weird state.
        version: String,
    }
);

impl Default for WorkspaceGraphHash<'_> {
    fn default() -> Self {
        WorkspaceGraphHash {
            projects: BTreeMap::default(),
            configs: BTreeMap::default(),
            env: BTreeMap::default(),
            in_docker: is_docker(),
            version: GlobalEnvBag::instance()
                .get("MOON_VERSION")
                .unwrap_or_default(),
        }
    }
}

impl<'graph> WorkspaceGraphHash<'graph> {
    pub fn add_projects(&mut self, projects: &'graph FxHashMap<Id, ProjectBuildData>) {
        self.projects.extend(projects.iter());
    }

    pub fn add_configs(&mut self, configs: BTreeMap<WorkspaceRelativePathBuf, String>) {
        self.configs.extend(configs);
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
