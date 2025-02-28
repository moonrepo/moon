use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::CliSession;
use moon_bun_tool::BunTool;
use moon_common::Id;
use moon_config::PlatformType;
use moon_deno_tool::DenoTool;
use moon_node_lang::PackageJsonCache;
use moon_node_tool::NodeTool;
use moon_platform::PlatformManager;
use moon_rust_tool::RustTool;
use moon_tool::DependencyManager;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_utils::{fs, json};
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn prune_bun(
    bun: &BunTool,
    session: &CliSession,
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
    session: &CliSession,
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
    session: &CliSession,
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
pub async fn prune_rust(_rust: &RustTool, session: &CliSession) -> AppResult {
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
pub async fn prune(session: CliSession) -> AppResult {
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
    for toolchain_id in toolchains {
        if toolchain_id == "unknown" {
            // Will crash with "Platform unknown has not been enabled"
            continue;
        }

        let platform = PlatformManager::read().get_by_toolchain(&toolchain_id)?;

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
        }
    }

    Ok(None)
}
