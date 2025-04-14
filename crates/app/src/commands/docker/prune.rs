use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::MoonSession;
use moon_bun_tool::BunTool;
use moon_common::Id;
use moon_config::PlatformType;
use moon_deno_tool::DenoTool;
use moon_node_lang::PackageJsonCache;
use moon_node_tool::NodeTool;
use moon_pdk_api::PruneDockerInput;
use moon_platform::PlatformManager;
use moon_rust_tool::RustTool;
use moon_tool::DependencyManager;
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_utils::{fs, json};
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn prune_toolchain(
    session: &MoonSession,
    toolchain_registry: &ToolchainRegistry,
    toolchain: &ToolchainPlugin,
) -> AppResult {
    let project_graph = session.get_project_graph().await?;

    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        if let (Some(vendor_name), Some(manifest_name)) = (
            &toolchain.metadata.vendor_dir_name,
            &toolchain.metadata.manifest_file_name,
        ) {
            debug!(
                "Removing {} vendor directories ({})",
                toolchain.metadata.name, vendor_name
            );

            fs::remove_dir_all(session.workspace_root.join(vendor_name))?;

            for source in project_graph.sources().values() {
                let project_root = source.to_logical_path(&session.workspace_root);

                // Only remove if there's a sibling manifest
                if project_root.join(manifest_name).exists() {
                    fs::remove_dir_all(project_root.join(vendor_name))?;
                }
            }
        }
    }

    if session.workspace_config.docker.prune.install_toolchain_deps {
        // TODO
    }

    if toolchain.has_func("prune_docker").await {
        toolchain
            .call_func_without_output(
                "prune_docker",
                PruneDockerInput {
                    context: toolchain_registry.create_context(),
                    docker_config: session.workspace_config.docker.prune.clone(),
                },
            )
            .await?;
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_bun(
    bun: &BunTool,
    session: &MoonSession,
    manifest: &DockerManifest,
) -> AppResult {
    let project_graph = session.get_project_graph().await?;

    // Some package managers do not delete stale node modules
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        debug!("Removing Bun vendor directories (node_modules)");

        fs::remove_dir_all(session.workspace_root.join("node_modules"))?;

        for source in project_graph.sources().values() {
            fs::remove_dir_all(source.join("node_modules").to_path(&session.workspace_root))?;
        }
    }

    // Install production only dependencies for focused projects
    if session.workspace_config.docker.prune.install_toolchain_deps {
        let mut package_names = vec![];

        for project_id in &manifest.focused_projects {
            if let Some(source) = project_graph.sources().get(project_id) {
                if let Some(package_json) =
                    PackageJsonCache::read(source.to_path(&session.workspace_root))?
                {
                    if let Some(package_name) = package_json.data.name {
                        package_names.push(package_name);
                    }
                }
            }
        }

        debug!(
            packages = ?package_names,
            "Pruning Bun dependencies"
        );

        bun.install_focused_dependencies(&(), &package_names, true)
            .await?;
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_deno(
    deno: &DenoTool,
    session: &MoonSession,
    _manifest: &DockerManifest,
) -> AppResult {
    // noop
    if session.workspace_config.docker.prune.install_toolchain_deps {
        deno.install_focused_dependencies(&(), &[], true).await?;
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_node(
    node: &NodeTool,
    session: &MoonSession,
    manifest: &DockerManifest,
) -> AppResult {
    let project_graph = session.get_project_graph().await?;

    // Some package managers do not delete stale node modules
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        debug!("Removing Node.js vendor directories (node_modules)");

        fs::remove_dir_all(session.workspace_root.join("node_modules"))?;

        for source in project_graph.sources().values() {
            fs::remove_dir_all(source.join("node_modules").to_path(&session.workspace_root))?;
        }
    }

    // Install production only dependencies for focused projects
    if session.workspace_config.docker.prune.install_toolchain_deps {
        let mut package_names = vec![];

        for project_id in &manifest.focused_projects {
            if let Some(source) = project_graph.sources().get(project_id) {
                if let Some(package_json) =
                    PackageJsonCache::read(source.to_path(&session.workspace_root))?
                {
                    if let Some(package_name) = package_json.data.name {
                        package_names.push(package_name);
                    }
                }
            }
        }

        debug!(
            packages = ?package_names,
            "Pruning Node.js dependencies"
        );

        node.get_package_manager()
            .install_focused_dependencies(node, &package_names, true)
            .await?;
    }

    Ok(None)
}

// This assumes that the project was built in --release mode. Is this correct?
#[instrument(skip_all)]
pub async fn prune_rust(_rust: &RustTool, session: &MoonSession) -> AppResult {
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        let target_dir = &session.workspace_root.join("target");
        let lockfile_path = &session.workspace_root.join("Cargo.lock");

        // Only delete target if relative to `Cargo.lock`
        if target_dir.exists() && lockfile_path.exists() {
            debug!(
                target_dir = ?target_dir,
                "Deleting Rust target directory"
            );

            fs::remove_dir_all(target_dir)?;
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune(session: MoonSession) -> AppResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    let workspace_graph = session.get_workspace_graph().await?;
    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let mut toolchains = FxHashSet::<Id>::default();

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Pruning dependencies for focused projects"
    );

    for project_id in &manifest.focused_projects {
        toolchains.extend(workspace_graph.get_project(project_id)?.toolchains.clone());
    }

    // Do this later so we only run once for each platform instead of per project
    let toolchain_registry = session.get_toolchain_registry().await?;

    for toolchain_id in toolchains {
        if toolchain_id == "unknown" {
            // Will crash with "Platform unknown has not been enabled"
            continue;
        }

        if let Ok(platform) = PlatformManager::read().get_by_toolchain(&toolchain_id) {
            match platform.get_type() {
                PlatformType::Bun => {
                    prune_bun(
                        platform
                            .get_tool()?
                            .as_any()
                            .downcast_ref::<BunTool>()
                            .unwrap(),
                        &session,
                        &manifest,
                    )
                    .await?;
                }
                PlatformType::Deno => {
                    prune_deno(
                        platform
                            .get_tool()?
                            .as_any()
                            .downcast_ref::<DenoTool>()
                            .unwrap(),
                        &session,
                        &manifest,
                    )
                    .await?;
                }
                PlatformType::Node => {
                    prune_node(
                        platform
                            .get_tool()?
                            .as_any()
                            .downcast_ref::<NodeTool>()
                            .unwrap(),
                        &session,
                        &manifest,
                    )
                    .await?;
                }
                PlatformType::Rust => {
                    prune_rust(
                        platform
                            .get_tool()?
                            .as_any()
                            .downcast_ref::<RustTool>()
                            .unwrap(),
                        &session,
                    )
                    .await?;
                }
                _ => {}
            };
        }

        if let Ok(toolchain) = toolchain_registry.load(toolchain_id).await {
            prune_toolchain(&session, &toolchain_registry, &toolchain).await?;
        }
    }

    Ok(None)
}
