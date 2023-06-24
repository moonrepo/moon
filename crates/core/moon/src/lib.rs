use moon_deno_platform::DenoPlatform;
use moon_dep_graph::DepGraphBuilder;
use moon_error::MoonError;
use moon_node_platform::NodePlatform;
use moon_project_graph::{ProjectGraph, ProjectGraphBuilder};
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use moon_utils::{is_ci, is_test_env};
use moon_workspace::{Workspace, WorkspaceError};
use starbase_utils::json;
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static TELEMETRY: AtomicBool = AtomicBool::new(true);
static TELEMETRY_READY: AtomicBool = AtomicBool::new(false);

pub fn is_telemetry_enabled() -> bool {
    while !TELEMETRY_READY.load(Ordering::Acquire) {
        continue;
    }

    TELEMETRY.load(Ordering::Relaxed)
}

pub fn set_telemetry(state: bool) {
    TELEMETRY.store(state, Ordering::Relaxed);
    TELEMETRY_READY.store(true, Ordering::Release);
}

/// Loads the workspace from the current working directory.
pub async fn load_workspace() -> miette::Result<Workspace> {
    let current_dir = env::current_dir().map_err(|_| WorkspaceError::MissingWorkingDir)?;
    let mut workspace = load_workspace_from(&current_dir).await?;

    if !is_test_env() {
        if workspace.vcs.is_enabled() {
            if let Ok(slug) = workspace.vcs.get_repository_slug().await {
                env::set_var("MOON_REPO_SLUG", slug);
            }
        }

        if is_ci() {
            workspace.signin_to_moonbase().await?;
        }
    }

    Ok(workspace)
}

/// Loads the workspace from a provided directory.
pub async fn load_workspace_from(path: &Path) -> miette::Result<Workspace> {
    let mut workspace = match Workspace::load_from(path) {
        Ok(workspace) => {
            set_telemetry(workspace.config.telemetry);
            workspace
        }
        Err(err) => {
            set_telemetry(false);
            return Err(err.into());
        }
    };

    if let Some(deno_config) = &workspace.toolchain_config.deno {
        workspace.register_platform(Box::new(DenoPlatform::new(
            deno_config,
            &workspace.toolchain_config.typescript,
            &workspace.root,
        )));
    }

    if let Some(node_config) = &workspace.toolchain_config.node {
        workspace.register_platform(Box::new(NodePlatform::new(
            node_config,
            &workspace.toolchain_config.typescript,
            &workspace.root,
        )));
    }

    if let Some(rust_config) = &workspace.toolchain_config.rust {
        workspace.register_platform(Box::new(RustPlatform::new(rust_config, &workspace.root)));
    }

    // Should be last since it's the most common
    workspace.register_platform(Box::<SystemPlatform>::default());

    Ok(workspace)
}

// Some commands require the toolchain to exist, but don't use
// the action pipeline. This is a simple flow to wire up the tools.
pub async fn load_workspace_with_toolchain() -> miette::Result<Workspace> {
    let mut workspace = load_workspace().await?;

    for platform in workspace.platforms.list_mut() {
        platform
            .setup_toolchain()
            .await
            .map_err(|e| WorkspaceError::Moon(MoonError::Generic(e.to_string())))?;
    }

    Ok(workspace)
}

pub fn build_dep_graph<'g>(
    workspace: &'g Workspace,
    project_graph: &'g ProjectGraph,
) -> DepGraphBuilder<'g> {
    DepGraphBuilder::new(&workspace.platforms, project_graph)
}

pub async fn build_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraphBuilder> {
    ProjectGraphBuilder::new(workspace).await
}

pub async fn generate_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraph> {
    let cache_path = workspace.cache.get_state_path("projectGraph.json");
    let mut builder = build_project_graph(workspace).await?;

    if builder.is_cached && cache_path.exists() {
        let graph: ProjectGraph = json::read_file(&cache_path)?;

        return Ok(graph);
    }

    builder.load_all()?;

    let graph = builder.build()?;

    if !builder.hash.is_empty() {
        json::write_file(&cache_path, &graph, false)?;
    }

    Ok(graph)
}
