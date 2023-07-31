use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{is_docker_container, Id};
use moon_hash::{content_hashable, ContentHashable};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::env;

content_hashable!(
    pub struct ProjectGraphHash<'graph> {
        // Data derived from the project graph builder.
        aliases: BTreeMap<&'graph String, &'graph Id>,
        sources: BTreeMap<&'graph Id, &'graph WorkspaceRelativePathBuf>,

        // Project and workspace configs required for cache invalidation.
        configs: BTreeMap<WorkspaceRelativePathBuf, String>,

        // The project graph stores absolute file paths, which breaks moon when
        // running tasks inside and outside of a container at the same time.
        // This flag helps to continuously bust the cache.
        in_container: bool,

        // Version of the moon CLI. We need to include this so that the graph
        // cache is invalidated between each release, otherwise internal Rust
        // changes (in project or task crates) are not reflected until the cache
        // is invalidated, which puts the program in a weird state.
        version: String,
    }
);

impl<'cfg> ProjectGraphHash<'cfg> {
    pub fn new() -> Self {
        ProjectGraphHash {
            aliases: BTreeMap::default(),
            sources: BTreeMap::default(),
            configs: BTreeMap::default(),
            in_container: is_docker_container(),
            version: env::var("MOON_VERSION").unwrap_or_default(),
        }
    }

    pub fn add_aliases(&mut self, aliases: &'cfg FxHashMap<String, Id>) {
        self.aliases.extend(aliases.iter().map(|(k, v)| (k, v)));
    }

    pub fn add_configs(&mut self, configs: BTreeMap<WorkspaceRelativePathBuf, String>) {
        self.configs.extend(configs);
    }

    pub fn add_sources(&mut self, sources: &'cfg FxHashMap<Id, WorkspaceRelativePathBuf>) {
        self.sources.extend(sources.iter().map(|(k, v)| (k, v)));
    }
}

impl<'graph> ContentHashable for ProjectGraphHash<'graph> {}
