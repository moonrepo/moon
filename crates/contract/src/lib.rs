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
    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform's ecosystem.
    fn load_project_graph_aliases(
        &self,
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
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        project_id: &str,
        project_root: &Path,
        project_config: &ProjectConfig,
    ) -> Result<TasksConfigsMap, MoonError> {
        Ok(BTreeMap::new())
    }
}

pub type RegisteredPlatforms = Vec<Box<dyn Platform>>;

pub trait Platformable {
    fn register_platform(&mut self, platform: Box<dyn Platform>) -> Result<(), MoonError>;
}

#[derive(Clone, Eq, PartialEq)]
pub enum SupportedPlatform {
    Node,
    System,
}

impl SupportedPlatform {
    pub fn label(&self) -> String {
        match self {
            SupportedPlatform::Node => "Node.js".into(),
            SupportedPlatform::System => "system".into(),
        }
    }
}

impl fmt::Display for SupportedPlatform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SupportedPlatform::Node => write!(f, "Node"),
            SupportedPlatform::System => write!(f, "System"),
        }
    }
}
