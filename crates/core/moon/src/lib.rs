use moon_action_graph::ActionGraphBuilder;
use moon_bun_platform::BunPlatform;
use moon_deno_platform::DenoPlatform;
use moon_node_platform::NodePlatform;
use moon_platform::{PlatformManager, PlatformType};
use moon_project_graph::{
    ExtendProjectData, ExtendProjectEvent, ExtendProjectGraphData, ExtendProjectGraphEvent,
    ProjectGraph, ProjectGraphBuilder, ProjectGraphBuilderContext,
};
use moon_rust_platform::RustPlatform;
use moon_system_platform::SystemPlatform;
use moon_utils::{is_ci, is_test_env};
use moon_workspace::Workspace;
use proto_core::ProtoEnvironment;
use starbase_events::{Emitter, EventState};
use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// Loads the workspace from the current environment.
pub async fn load_workspace_from(proto_env: Arc<ProtoEnvironment>) -> miette::Result<Workspace> {
    let mut workspace = match Workspace::load_from(&proto_env.cwd, &proto_env) {
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

    if let Some(bun_config) = &workspace.toolchain_config.bun {
        registry.register(
            PlatformType::Bun,
            Box::new(BunPlatform::new(
                bun_config,
                &workspace.toolchain_config.typescript,
                &workspace.root,
                Arc::clone(&proto_env),
            )),
        );
    }

    if let Some(deno_config) = &workspace.toolchain_config.deno {
        registry.register(
            PlatformType::Deno,
            Box::new(DenoPlatform::new(
                deno_config,
                &workspace.toolchain_config.typescript,
                &workspace.root,
                Arc::clone(&proto_env),
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
                Arc::clone(&proto_env),
            )),
        );
    }

    if let Some(rust_config) = &workspace.toolchain_config.rust {
        registry.register(
            PlatformType::Rust,
            Box::new(RustPlatform::new(
                rust_config,
                &workspace.root,
                Arc::clone(&proto_env),
            )),
        );
    }

    // Should be last since it's the most common
    registry.register(
        PlatformType::System,
        Box::new(SystemPlatform::new(&workspace.root, Arc::clone(&proto_env))),
    );

    if !is_test_env() {
        if workspace.vcs.is_enabled() {
            if let Ok(slug) = workspace.vcs.get_repository_slug().await {
                env::set_var("MOONBASE_REPO_SLUG", slug);
            }
        }

        if is_ci() {
            workspace.signin_to_moonbase().await?;
        }
    }

    Ok(workspace)
}

pub async fn load_workspace_from_sandbox(sandbox: &Path) -> miette::Result<Workspace> {
    load_workspace_from(Arc::new(ProtoEnvironment::new_testing(sandbox))).await
}

pub async fn load_toolchain() -> miette::Result<()> {
    for platform in PlatformManager::write().list_mut() {
        platform.setup_toolchain().await?;
    }

    Ok(())
}

pub fn build_action_graph(project_graph: &ProjectGraph) -> miette::Result<ActionGraphBuilder> {
    ActionGraphBuilder::new(project_graph)
}

pub async fn create_project_graph_context(workspace: &Workspace) -> ProjectGraphBuilderContext {
    let context = ProjectGraphBuilderContext {
        extend_project: Emitter::<ExtendProjectEvent>::new(),
        extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
        inherited_tasks: &workspace.tasks_config,
        toolchain_config: &workspace.toolchain_config,
        vcs: Some(&workspace.vcs),
        working_dir: &workspace.working_dir,
        workspace_config: &workspace.config,
        workspace_root: &workspace.root,
    };

    context
        .extend_project
        .on(
            |event: Arc<ExtendProjectEvent>, data: Arc<RwLock<ExtendProjectData>>| async move {
                let mut data = data.write().await;

                for platform in PlatformManager::read().list() {
                    data.dependencies
                        .extend(platform.load_project_implicit_dependencies(
                            &event.project_id,
                            event.project_source.as_str(),
                        )?);

                    data.tasks.extend(
                        platform
                            .load_project_tasks(&event.project_id, event.project_source.as_str())?,
                    );
                }

                Ok(EventState::Continue)
            },
        )
        .await;

    context
        .extend_project_graph
        .on(|event: Arc<ExtendProjectGraphEvent>, data: Arc<RwLock<ExtendProjectGraphData>>| async move {
            let mut data = data.write().await;


            for platform in PlatformManager::write().list_mut() {
                platform.load_project_graph_aliases(&event.sources, &mut data.aliases)?;
            }

            Ok(EventState::Continue)
        })
        .await;

    context
}

pub async fn build_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraphBuilder> {
    ProjectGraphBuilder::new(create_project_graph_context(workspace).await).await
}

pub async fn generate_project_graph(workspace: &mut Workspace) -> miette::Result<ProjectGraph> {
    let context = create_project_graph_context(workspace).await;
    let builder =
        ProjectGraphBuilder::generate(context, &workspace.cache_engine, &workspace.hash_engine)
            .await?;

    let graph = builder.build().await?;

    Ok(graph)
}
