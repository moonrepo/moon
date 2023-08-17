use async_trait::async_trait;
use moon_action_context::ActionContext;
use moon_common::Id;
use moon_config::{
    DependencyConfig, HasherConfig, PlatformType, ProjectConfig, ProjectsAliasesMap,
    ProjectsSourcesMap, TasksConfigsMap,
};
use moon_hash::ContentHasher;
use moon_platform_runtime::{Runtime, Version};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_tool::Tool;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

#[async_trait]
pub trait Platform: Debug + Send + Sync {
    /// Return the type of this platform.
    fn get_type(&self) -> PlatformType;

    /// Return a runtime with an appropriate version based on the provided configs.
    fn get_runtime_from_config(&self, project_config: Option<&ProjectConfig>) -> Runtime;

    /// Return true if the current platform is for the provided project or runtime.
    fn matches(&self, platform: &PlatformType, runtime: Option<&Runtime>) -> bool;

    // PROJECT GRAPH

    /// Determine if the provided project is within the platform's dependency manager
    /// workspace (not to be confused with moon's workspace).
    fn is_project_in_dependency_workspace(&self, project_source: &str) -> miette::Result<bool> {
        Ok(false)
    }

    /// During project graph creation, load project aliases for the resolved
    /// map of projects that are unique to the platform's ecosystem.
    fn load_project_graph_aliases(
        &mut self,
        projects_map: &ProjectsSourcesMap,
        aliases_map: &mut ProjectsAliasesMap,
    ) -> miette::Result<()> {
        Ok(())
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// scan for any implicit project dependency relations using the platforms manifest.
    fn load_project_implicit_dependencies(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<Vec<DependencyConfig>> {
        Ok(vec![])
    }

    /// During project creation (when being lazy loaded and instantiated in the graph),
    /// load and infer any *additional* tasks for the platform.
    fn load_project_tasks(
        &self,
        project_id: &str,
        project_source: &str,
    ) -> miette::Result<TasksConfigsMap> {
        Ok(BTreeMap::new())
    }

    // TOOLCHAIN

    fn is_toolchain_enabled(&self) -> miette::Result<bool>;

    /// Return a tool instance from the internal toolchain for the top-level version.
    /// If the version does not exist in the toolchain, return an error.
    fn get_tool(&self) -> miette::Result<Box<&dyn Tool>>;

    /// Return a tool instance from the internal toolchain for the provided version.
    /// If the version does not exist in the toolchain, return an error.
    fn get_tool_for_version(&self, version: Version) -> miette::Result<Box<&dyn Tool>>;

    /// Return the filename of the lockfile and manifest (in this order)
    /// for the language's dependency manager, if applicable.
    fn get_dependency_configs(&self) -> miette::Result<Option<(String, String)>> {
        Ok(None)
    }

    /// Setup the top-level tool in the toolchain if applicable.
    /// This is a one off flow, as most flows will be using the pipeline.
    async fn setup_toolchain(&mut self) -> miette::Result<()> {
        Ok(())
    }

    /// Teardown all tools that are currently registered in the toolchain.
    async fn teardown_toolchain(&mut self) -> miette::Result<()> {
        Ok(())
    }

    // ACTIONS

    /// Setup a tool by registering it into the toolchain with the provided version
    /// (if it hasn't already been registered). Once registered, download and install.
    /// Return a count of how many tools were installed.
    async fn setup_tool(
        &mut self,
        context: &ActionContext,
        runtime: &Runtime,
        last_versions: &mut FxHashMap<String, String>,
    ) -> miette::Result<u8> {
        Ok(0)
    }

    /// Install dependencies in the target working directory with a tool and its
    /// dependency manager using the provided version.
    async fn install_deps(
        &self,
        context: &ActionContext,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<()> {
        Ok(())
    }

    /// Sync a project (and its dependencies) when applicable.
    /// Return true if any files were modified as a result of syncing.
    async fn sync_project(
        &self,
        context: &ActionContext,
        project: &Project,
        dependencies: &FxHashMap<Id, Arc<Project>>,
    ) -> miette::Result<bool> {
        Ok(false)
    }

    /// Hash all dependencies and their versions from the provided manifest file.
    /// These will be used to determine whether to install dependencies or not.
    async fn hash_manifest_deps(
        &self,
        manifest_path: &Path,
        hasher: &mut ContentHasher,
        hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        Ok(())
    }

    /// Hash information related to running a target (project task), that isn't
    /// part of the default target hashing strategy.
    async fn hash_run_target(
        &self,
        project: &Project,
        runtime: &Runtime,
        hasher: &mut ContentHasher,
        hasher_config: &HasherConfig,
    ) -> miette::Result<()> {
        Ok(())
    }

    /// Create an async command to run a target's child process.
    async fn create_run_target_command(
        &self,
        context: &ActionContext,
        project: &Project,
        task: &Task,
        runtime: &Runtime,
        working_dir: &Path,
    ) -> miette::Result<Command>;
}
