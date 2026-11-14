use crate::inherited_tasks_config::*;
use crate::shapes::{Input, OneOrMany};
use miette::IntoDiagnostic;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, standardize_separators};
use rustc_hash::{FxBuildHasher, FxHashMap};
use schematic::schema::indexmap::IndexMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InheritedTasksEntry {
    pub input: WorkspaceRelativePathBuf,
    pub config: InheritedTasksConfig,
}

/// Inherited configs derived from a distinct set of matched entries,
/// keyed by the ordered source paths of those entries.
type InheritedTasksCache = scc::HashMap<Vec<String>, Arc<InheritedTasks>, FxBuildHasher>;

#[derive(Default)]
pub struct InheritedTasksManager {
    pub configs: Vec<InheritedTasksEntry>,

    // Many projects match the same set of configs, so derive each
    // distinct set once and share it, instead of cloning per project
    cache: InheritedTasksCache,
}

impl Clone for InheritedTasksManager {
    fn clone(&self) -> Self {
        Self {
            configs: self.configs.clone(),
            cache: InheritedTasksCache::default(),
        }
    }
}

impl fmt::Debug for InheritedTasksManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InheritedTasksManager")
            .field("configs", &self.configs)
            .finish_non_exhaustive()
    }
}

impl InheritedTasksManager {
    pub fn add_config(
        &mut self,
        workspace_root: &Path,
        config_path: &Path,
        config: InheritedTasksConfig,
    ) -> miette::Result<()> {
        self.configs.push(InheritedTasksEntry {
            input: config_path.relative_to(workspace_root).into_diagnostic()?,
            config,
        });

        // Derived configs may now be stale
        self.cache.clear_sync();

        Ok(())
    }

    pub fn get_inherited_config(&self, input: InheritFor) -> miette::Result<Arc<InheritedTasks>> {
        self.get_or_create_inherited_config(self.match_inherited_configs_in_order(input))
    }

    pub fn get_inherited_config_from_sources(
        &self,
        sources: &[String],
    ) -> miette::Result<Arc<InheritedTasks>> {
        self.get_or_create_inherited_config(
            sources
                .iter()
                .filter_map(|source| {
                    self.configs
                        .iter()
                        .find(|entry| standardize_separators(entry.input.as_str()) == *source)
                })
                .collect(),
        )
    }

    fn get_or_create_inherited_config(
        &self,
        entries: Vec<&InheritedTasksEntry>,
    ) -> miette::Result<Arc<InheritedTasks>> {
        // The derived config is determined by the ordered entries alone
        let key = entries
            .iter()
            .map(|entry| standardize_separators(entry.input.as_str()))
            .collect::<Vec<_>>();

        if let Some(config) = self.cache.read_sync(&key, |_, config| Arc::clone(config)) {
            return Ok(config);
        }

        // Create outside of the bucket lock, and if another thread won the
        // race, use its value so that all consumers share the same instance
        let config = Arc::new(Self::create_inherited_config(entries)?);

        Ok(Arc::clone(
            self.cache.entry_sync(key).or_insert_with(|| config).get(),
        ))
    }

    fn create_inherited_config(
        entries: Vec<&InheritedTasksEntry>,
    ) -> miette::Result<InheritedTasks> {
        let mut configs = IndexMap::default();
        let mut layers = FxHashMap::<String, Vec<String>>::default();

        for config_entry in entries {
            let source_path = standardize_separators(config_entry.input.as_str());
            let mut config = config_entry.config.clone();

            let default_toolchain = config
                .inherited_by
                .as_ref()
                .and_then(|by| by.default_toolchain());

            for (task_id, task) in config.tasks.iter_mut() {
                // Automatically set this source as an input
                task.global_inputs
                    .push(Input::parse(format!("/{source_path}"))?);

                // Automatically set the toolchain
                if task.toolchains.is_none()
                    && let Some(toolchain) = &default_toolchain
                {
                    task.toolchains = Some(OneOrMany::One(toolchain.to_owned()));
                }

                // Keep track of what layers a task inherited
                layers
                    .entry(task_id.to_string())
                    .or_default()
                    .push(source_path.clone());
            }

            configs.insert(source_path, config);
        }

        Ok(InheritedTasks { configs, layers })
    }

    pub fn match_inherited_configs_in_order(&self, input: InheritFor) -> Vec<&InheritedTasksEntry> {
        let mut entries = self
            .configs
            .iter()
            .filter(|entry| {
                match &entry.config.inherited_by {
                    Some(by) => by.matches(&input),
                    // If no `inheritedBy` setting, then it's inherited by all!
                    None => true,
                }
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|entry| {
            (
                entry
                    .config
                    .inherited_by
                    .as_ref()
                    .map_or(0, |by| by.order()),
                entry.input.as_str(),
            )
        });

        entries
    }

    pub fn should_invalidate(&self, other: &Self) -> bool {
        self.configs != other.configs
    }
}
