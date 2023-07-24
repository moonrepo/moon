use moon_deno_platform::DenoPlatform;
use moon_dep_graph::DepGraphBuilder;
use moon_hash::HashEngine;
use moon_node_platform::NodePlatform;
use moon_platform::{PlatformManager, PlatformType};
use moon_project_graph2::{
    DetectLanguageEvent, DetectPlatformEvent, ExtendProjectEvent, ExtendProjectGraphEvent,
    ProjectGraph, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use moon_utils::{is_ci, is_test_env};
use moon_workspace::{Workspace, WorkspaceError};
use starbase_events::Emitter;
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
    let workspace = match Workspace::load_from(path) {
        Ok(workspace) => {
            set_telemetry(workspace.config.telemetry);
            workspace
        }
        Err(err) => {
            set_telemetry(false);
            return Err(err);
        }
    };

    let registry = PlatformManager::write();

    // Primarily for testing
    registry.reset();

    if let Some(deno_config) = &workspace.toolchain_config.deno {
        registry.register(
            PlatformType::Deno,
            Box::new(DenoPlatform::new(
                deno_config,
                &workspace.toolchain_config.typescript,
                &workspace.root,
            )),
        );
    }

    if let Some(node_config) = &workspace.toolchain_config.node {
        registry.register(
            PlatformType::Node,
            Box::new(NodePlatform::new(
                node_config,
                &workspace.toolchain_config.typescript,
                &workspace.root,
            )),
        );
    }

    if let Some(rust_config) = &workspace.toolchain_config.rust {
        registry.register(
            PlatformType::Rust,
            Box::new(RustPlatform::new(rust_config, &workspace.root)),
        );
    }

    // Should be last since it's the most common
    registry.register(PlatformType::System, Box::<SystemPlatform>::default());

    Ok(workspace)
}

// Some commands require the toolchain to exist, but don't use
// the action pipeline. This is a simple flow to wire up the tools.
pub async fn load_workspace_with_toolchain() -> miette::Result<Workspace> {
    let workspace = load_workspace().await?;

    for platform in PlatformManager::write().list_mut() {
        platform.setup_toolchain().await?;
    }

    Ok(workspace)
}

pub fn build_dep_graph(project_graph: &ProjectGraph) -> DepGraphBuilder {
    DepGraphBuilder::new(project_graph)
}

pub fn create_project_graph_context(workspace: &Workspace) -> ProjectGraphBuilderContext {
    ProjectGraphBuilderContext {
        extend_project: Emitter::<ExtendProjectEvent>::new(),
        extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
        detect_language: Emitter::<DetectLanguageEvent>::new(),
        detect_platform: Emitter::<DetectPlatformEvent>::new(),
        inherited_tasks: &workspace.tasks_config,
        toolchain_config: &workspace.toolchain_config,
        vcs: &workspace.vcs,
        workspace_config: &workspace.config,
        workspace_root: &workspace.root,
    }
}

pub async fn build_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraphBuilder> {
    ProjectGraphBuilder::new(create_project_graph_context(workspace)).await
}

pub async fn generate_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraph> {
    let context = create_project_graph_context(workspace);
    let hash_engine = HashEngine::new(&workspace.cache.dir);
    let builder = ProjectGraphBuilder::generate(context, &hash_engine, &workspace.cache).await?;
    let graph = builder.build().await?;

    Ok(graph)
}
