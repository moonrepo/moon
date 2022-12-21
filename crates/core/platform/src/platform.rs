use moon_config::{
    DependencyConfig, PlatformType, ProjectConfig, ProjectLanguage, ProjectsAliasesMap,
    ProjectsSourcesMap, TasksConfigsMap,
};
use moon_error::MoonError;
use moon_platform_runtime::Runtime;
use moon_project::Project;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::Path;

pub trait Platform: Debug + Send + Sync {
    /// Return the type of this platform.
    fn get_type(&self) -> PlatformType;

    /// Return a runtime with an appropriate version based on the provided configs.
    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Option<Runtime>;

    /// Determine if the provided project is within the platform's dependency manager
    /// workspace (not to be confused with moon's workspace).
    fn is_project_in_dependency_workspace(&self, project: &Project) -> Result<bool, MoonError> {
        Ok(true)
    }

    /// Determine the language of project at the provided path by locating
    /// and inspecting manifest or config files.
    fn is_project_language(&self, project_root: &Path) -> Option<ProjectLanguage> {
        None
    }

    /// Determine if the command of a task applies to the current platform.
    fn is_task_command(&self, command: &str) -> bool {
        false
    }

    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform's ecosystem.
    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> Result<(), MoonError> {
        Ok(())
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// scan for any implicit project dependency relations using the platforms manifest.
    fn load_project_implicit_dependencies(
        &self,
        project: &Project,
        aliases_map: &ProjectsAliasesMap,
    ) -> Result<Vec<DependencyConfig>, MoonError> {
        Ok(vec![])
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// load and infer any *additional* tasks for the platform.
    fn load_project_tasks(&self, project: &Project) -> Result<TasksConfigsMap, MoonError> {
        Ok(BTreeMap::new())
    }

    /// Return true if the current platform is for the provided project or runtime.
    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool;
}
