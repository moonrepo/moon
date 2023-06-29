use super::MANIFEST_NAME;
use crate::commands::docker::scaffold::DockerManifest;
use moon::{generate_project_graph, load_workspace_with_toolchain};
use moon_config::PlatformType;
use moon_node_lang::{PackageJson, NODE};
use moon_node_tool::NodeTool;
use moon_project_graph::ProjectGraph;
use moon_rust_lang::{CARGO, RUST};
use moon_rust_tool::RustTool;
use moon_terminal::safe_exit;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_utils::fs;
use starbase_utils::json;
use std::path::Path;

pub async fn prune_node(
    node: &NodeTool,
    workspace_root: &Path,
    project_graph: &ProjectGraph,
    manifest: &DockerManifest,
) -> AppResult {
    let mut package_names = vec![];

    for project_id in &manifest.focused_projects {
        if let Some(project_source) = project_graph.sources.get(project_id) {
            if let Some(package_json) = PackageJson::read(workspace_root.join(project_source))? {
                if let Some(package_name) = package_json.name {
                    package_names.push(package_name);
                }
            }
        }
    }

    // Some package managers do not delete stale node modules
    if let Some(vendor_dir) = NODE.vendor_dir {
        fs::remove_dir_all(workspace_root.join(vendor_dir))?;

        for project_source in project_graph.sources.values() {
            fs::remove_dir_all(workspace_root.join(project_source).join(vendor_dir))?;
        }
    }

    // Install production only dependencies for focused projects
    node.get_package_manager()
        .install_focused_dependencies(node, &package_names, true)
        .await?;

    Ok(())
}

// This assumes that the project was built in --release mode. Is this correct?
pub async fn prune_rust(_rust: &RustTool, workspace_root: &Path) -> AppResult {
    let target_dir = workspace_root.join(RUST.vendor_dir.unwrap());
    let lockfile_path = workspace_root.join(CARGO.lockfile);

    // Only delete target if relative to `Cargo.lock`
    if target_dir.exists() && lockfile_path.exists() {
        fs::remove_dir_all(target_dir)?;
    }

    Ok(())
}

pub async fn prune() -> AppResult {
    let mut workspace = load_workspace_with_toolchain().await?;
    let manifest_path = workspace.root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        eprintln!("Unable to prune, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?");
        safe_exit(1);
    }

    let project_graph = generate_project_graph(&mut workspace).await?;
    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let mut platforms = FxHashSet::<PlatformType>::default();

    for project_id in &manifest.focused_projects {
        platforms.insert(project_graph.get(project_id)?.language.clone().into());
    }

    // Do this later so we only run once for each platform instead of per project
    for platform_type in platforms {
        let platform = workspace.platforms.get(platform_type)?;

        match platform.get_type() {
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
            PlatformType::Deno | PlatformType::System | PlatformType::Unknown => {}
        }
    }

    Ok(())
}