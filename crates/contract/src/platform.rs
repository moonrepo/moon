#![allow(unused_variables)]

use moon_config::{
    DependencyConfig, ProjectConfig, ProjectsAliasesMap, ProjectsSourcesMap, TasksConfigsMap,
    WorkspaceConfig,
};
use moon_error::MoonError;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub trait Platform: Send + Sync {
    /// Return true if the current platform instance is for the supported platform enum.
    fn is(&self, platform: &Runtime) -> bool;

    /// Determine if the provided project is within the platform's package manager
    /// workspace (not to be confused with moon's workspace).
    fn is_project_in_package_manager_workspace(
        &self,
        project_id: &str,
        project_root: &Path,
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
    ) -> Result<bool, MoonError> {
        Ok(true)
    }

    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform's ecosystem.
    fn load_project_graph_aliases(
        &mut self,
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        Ok(())
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// scan for any implicit project dependency relations using the platforms manifest.
    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_root: &Path,
        project_config: &ProjectConfig,
        aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        Ok(vec![])
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// load and infer any *additional* tasks for the platform.
    fn load_project_tasks(
        &self,
        project_id: &str,
        project_root: &Path,
        project_config: &ProjectConfig,
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
    ) -> Result<TasksConfigsMap, MoonError> {
        Ok(BTreeMap::new())
    }
}

pub type RegisteredPlatforms = Vec<Box<dyn Platform>>;

pub trait Platformable {
    fn register_platform(&mut self, platform: Box<dyn Platform>) -> Result<(), MoonError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Runtime {
    Node(String),
    System,
}

impl Runtime {
    pub fn label(&self) -> String {
        match self {
            Runtime::Node(version) => format!("Node.js v{}", version),
            Runtime::System => "system".into(),
        }
    }

    pub fn version(&self) -> String {
        match self {
            Runtime::Node(version) => version.into(),
            Runtime::System => "latest".into(),
        }
    }
}

impl fmt::Display for Runtime {
    // Primarily used in action graph node labels
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Runtime::Node(_) => write!(f, "Node"),
            Runtime::System => write!(f, "System"),
        }
    }
}
