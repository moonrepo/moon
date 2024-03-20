use super::MANIFEST_NAME;
use crate::commands::docker::scaffold::DockerManifest;
use miette::miette;
use moon::generate_project_graph;
use moon_bun_tool::BunTool;
use moon_config::PlatformType;
use moon_deno_tool::DenoTool;
use moon_node_lang::PackageJson;
use moon_node_tool::NodeTool;
use moon_platform::PlatformManager;
use moon_project_graph::ProjectGraph;
use moon_rust_tool::RustTool;
use moon_tool::DependencyManager;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use starbase::system;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::fs;
use starbase_utils::json;
use std::path::Path;

pub async fn prune_bun(
    bun: &BunTool,
    workspace_root: &Path,
    project_graph: &ProjectGraph,
    manifest: &DockerManifest,
) -> AppResult {
    let mut package_names = vec![];

    for project_id in &manifest.focused_projects {
        if let Some(source) = project_graph.sources().get(project_id) {
            if let Some(package_json) = PackageJson::read(source.to_path(workspace_root))? {
                if let Some(package_name) = package_json.name {
                    package_names.push(package_name);
                }
            }
        }
    }

    // Some package managers do not delete stale node modules
    fs::remove_dir_all(workspace_root.join("node_modules"))?;

    for source in project_graph.sources().values() {
        fs::remove_dir_all(source.join("node_modules").to_path(workspace_root))?;
    }

    // Install production only dependencies for focused projects
    bun.install_focused_dependencies(&(), &package_names, true)
        .await?;

    Ok(())
}

pub async fn prune_deno(
    deno: &DenoTool,
    _workspace_root: &Path,
    _project_graph: &ProjectGraph,
    _manifest: &DockerManifest,
) -> AppResult {
    deno.install_focused_dependencies(&(), &[], true).await?;

    Ok(())
}

pub async fn prune_node(
    node: &NodeTool,
    workspace_root: &Path,
    project_graph: &ProjectGraph,
    manifest: &DockerManifest,
) -> AppResult {
    let mut package_names = vec![];

    for project_id in &manifest.focused_projects {
        if let Some(source) = project_graph.sources().get(project_id) {
            if let Some(package_json) = PackageJson::read(source.to_path(workspace_root))? {
                if let Some(package_name) = package_json.name {
                    package_names.push(package_name);
                }
            }
        }
    }

    // Some package managers do not delete stale node modules
    fs::remove_dir_all(workspace_root.join("node_modules"))?;

    for source in project_graph.sources().values() {
        fs::remove_dir_all(source.join("node_modules").to_path(workspace_root))?;
    }

    // Install production only dependencies for focused projects
    node.get_package_manager()
        .install_focused_dependencies(node, &package_names, true)
        .await?;

    Ok(())
}

// This assumes that the project was built in --release mode. Is this correct?
pub async fn prune_rust(_rust: &RustTool, workspace_root: &Path) -> AppResult {
    let target_dir = workspace_root.join("target");
    let lockfile_path = workspace_root.join("Cargo.lock");

    // Only delete target if relative to `Cargo.lock`
    if target_dir.exists() && lockfile_path.exists() {
        fs::remove_dir_all(target_dir)?;
    }

    Ok(())
}

#[system]
pub async fn prune(workspace: ResourceMut<Workspace>) {
    let manifest_path = workspace.root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(miette!(
            code = "moon::docker::prune",
            "Unable to prune, docker manifest missing. Has it been scaffolded with {}?",
            color::shell("moon docker scaffold")
        ));
    }

    let project_graph = generate_project_graph(workspace).await?;
    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let mut platforms = FxHashSet::<PlatformType>::default();

    for project_id in &manifest.focused_projects {
        platforms.insert(project_graph.get(project_id)?.language.clone().into());
    }

    // Do this later so we only run once for each platform instead of per project
    for platform_type in platforms {
        if platform_type.is_unknown() {
            // Unknown will crash with "Platform unknown has not been enabled"
            continue;
        }

        let platform = PlatformManager::read().get(platform_type)?;

        match platform.get_type() {
            PlatformType::Bun => {
                prune_bun(
                    platform
                        .get_tool()?
                        .as_any()
                        .downcast_ref::<BunTool>()
                        .unwrap(),
                    &workspace.root,
                    &project_graph,
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
                    &workspace.root,
                    &project_graph,
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
                    &workspace.root,
                    &project_graph,
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
                    &workspace.root,
                )
                .await?;
            }
            _ => {}
        }
    }
}
