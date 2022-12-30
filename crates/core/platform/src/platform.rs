use async_trait::async_trait;
use moon_action_context::ActionContext;
use moon_config::{
    DependencyConfig, HasherConfig, PlatformType, ProjectConfig, ProjectLanguage,
    ProjectsAliasesMap, ProjectsSourcesMap, TasksConfigsMap,
};
use moon_error::MoonError;
use moon_hasher::HashSet;
use moon_platform_runtime::{Runtime, Version};
use moon_project::{Project, ProjectError};
use moon_task::Task;
use moon_tool::{Tool, ToolError};
use moon_utils::process::Command;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::Path;

#[async_trait]
pub trait Platform: Debug + Send + Sync {
    /// Return the type of this platform.
    fn get_type(&self) -> PlatformType;

    /// Return a runtime with an appropriate version based on the provided configs.
    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Option<Runtime>;

    /// Return true if the current platform is for the provided project or runtime.
    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool;

    // PROJECT GRAPH

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

    // TOOLCHAIN

    /// Return a tool instance from the internal toolchain for the top-level version.
    /// If the version does not exist in the toolchain, return an error.
    fn get_tool(&self) -> Result<Box<&dyn Tool>, ToolError>;

    /// Return a tool instance from the internal toolchain for the provided version.
    /// If the version does not exist in the toolchain, return an error.
    fn get_tool_for_version(&self, version: Version) -> Result<Box<&dyn Tool>, ToolError>;

    /// Return the filename of the lockfile and manifest (in this order)
    /// for the language's dependency manager, if applicable.
    fn get_dependency_configs(&self) -> Result<Option<(String, String)>, ToolError> {
        Ok(None)
    }

    // ACTIONS

    /// Setup a tool by registering it into the toolchain with the provided version
    /// (if it hasn't already been registered). Once registered, download and install.
    /// Return a count of how many tools were installed.
    async fn setup_tool(
        &mut self,
        context: &ActionContext,
        tool_version: Version,
        last_versions: &mut FxHashMap<String, String>,
    ) -> Result<u8, ToolError> {
        Ok(0)
    }

    /// Install dependencies in the target working directory with a tool and its
    /// dependency manager using the provided version.
    async fn install_deps(
        &self,
        context: &ActionContext,
        tool_version: Version,
        working_dir: &Path,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    /// Sync a project (and its dependencies) when applicable.
    /// Return true if any files were modified as a result of syncing.
    async fn sync_project(
        &self,
        context: &ActionContext,
        project: &Project,
        dependencies: &FxHashMap<String, &Project>,
    ) -> Result<bool, ProjectError> {
        Ok(false)
    }

    /// Hash all dependencies and their versions from the provided manifest file.
    /// These will be used to determine whether to install dependencies or not.
    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hashset: &mut HashSet,
        hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    /// Hash information related to running a target (project task), that isn't
    /// part of the default target hashing strategy.
    async fn hash_run_target(
        &self,
        project: &Project,
        hashset: &mut HashSet,
        hasher_config: &HasherConfig,
    ) -> Result<(), ToolError> {
        Ok(())
    }

    async fn create_run_target_command(
        &self,
        context: &ActionContext,
        project: &Project,
        task: &Task,
        working_dir: &Path,
    ) -> Result<Command, ToolError>;
}
