#![allow(unused_variables)]

mod runtime;

use moon_config::{
    DependencyConfig, PlatformType, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap,
    TasksConfigsMap,
};
use moon_error::MoonError;
pub use runtime::{Runtime, Version};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::Path;

pub trait Platform: Debug + Send + Sync {
    /// Return the type of platform.
    fn get_type(&self) -> PlatformType;

    /// Return a runtime with an appropriate version based on the provided configs.
    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Option<Runtime>;

    /// Determine if the provided project is within the platform's package manager
    /// workspace (not to be confused with moon's workspace).
    fn is_project_in_package_manager_workspace(
        &self,
        project_id: &str,
        project_root: &Path,
    ) -> Result<bool, MoonError> {
        Ok(true)
    }

    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform's ecosystem.
    fn load_project_graph_aliases(
        &mut self,
        project_sources: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        Ok(())
    }

    /// During project creation within the project graph, find a matching alias
    /// for the previously loaded map of aliases, if applicable
    fn load_project_alias(&self, aliases_map: &ProjectsAliasesMap) -> Option<String> {
        None
    }

    /// During project creation within the project graph, scan for any implicit
    /// project dependency relations using the platforms manifest.
    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_root: &Path,
        project_config: &ProjectConfig,
        aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        Ok(vec![])
    }

    /// During project creation within the project graph, load and infer any
    /// *additional* tasks for the platform.
    fn load_project_tasks(
        &self,
        project_id: &str,
        project_root: &Path,
        project_config: &ProjectConfig,
    ) -> Result<TasksConfigsMap, MoonError> {
        Ok(BTreeMap::new())
    }

    /// Return true if the current platform is for the provided project or runtime.
    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool;
}

pub type BoxedPlatform = Box<dyn Platform>;

#[derive(Debug, Default)]
pub struct PlatformManager {
    cache: FxHashMap<PlatformType, BoxedPlatform>,
}

impl PlatformManager {
    pub fn find<T: Into<PlatformType>>(&self, type_of: T) -> Option<&BoxedPlatform> {
        let type_of = type_of.into();

        self.find_with(|platform| platform.matches(&type_of, None))
    }

    pub fn find_with<P>(&self, predicate: P) -> Option<&BoxedPlatform>
    where
        P: Fn(&&BoxedPlatform) -> bool,
    {
        self.cache.values().find(predicate)
    }

    pub fn get<T: Into<PlatformType>>(&self, type_of: T) -> Option<&BoxedPlatform> {
        let type_of = type_of.into();

        self.cache.get(&type_of)
    }

    pub fn list_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<PlatformType, BoxedPlatform> {
        self.cache.values_mut()
    }

    pub fn register(&mut self, type_of: PlatformType, platform: BoxedPlatform) {
        self.cache.insert(type_of, platform);
    }
}
