#![allow(unused_variables)]

use moon_config::{ProjectsAliasesMap, ProjectsSourcesMap, WorkspaceConfig};
use moon_error::MoonError;
use std::path::Path;

pub trait PlatformBridge {
    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform/language.
    fn load_project_aliases(
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        Ok(())
    }
}
