use crate::commands::docker::scaffold::DockerManifest;
use crate::helpers::AnyError;
use moon::{generate_project_graph, load_workspace_with_toolchain};
use moon_config::{PlatformType, ProjectLanguage};
use moon_node_lang::{PackageJson, NODE};
use moon_node_tool::NodeTool;
use moon_project_graph::ProjectGraph;
use moon_terminal::safe_exit;
use moon_utils::{fs, json};
use moon_workspace::Workspace;

pub async fn prune_node(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    manifest: &DockerManifest,
) -> Result<(), AnyError> {
    let mut package_names = vec![];

    for project_id in &manifest.focused_projects {
        if let Some(project_source) = project_graph.sources.get(project_id) {
            if let Some(package_json) = PackageJson::read(workspace.root.join(project_source))? {
                if let Some(package_name) = package_json.name {
                    package_names.push(package_name);
                }
            }
        }
    }

    // Some package managers do not delete stale node modules
    if let Some(vendor_dir) = NODE.vendor_dir {
        fs::remove_dir_all(workspace.root.join(vendor_dir))?;

        for project_source in project_graph.sources.values() {
            fs::remove_dir_all(workspace.root.join(project_source).join(vendor_dir))?;
        }
    }

    // Install production only dependencies for focused projects
    let node = workspace
        .platforms
        .get(PlatformType::Node)?
        .get_language_tool(None)?
        .as_any()
        .downcast_ref::<NodeTool>()
        .unwrap();

    node.get_package_manager()
        .install_focused_dependencies(node, &package_names, true)
        .await?;

    // Remove extraneous node module folders for unfocused projects
    // for project_id in &manifest.unfocused_projects {
    //     if let Some(project_source) = project_graph.sources.get(project_id) {
    //         fs::remove_dir_all(workspace.root.join(project_source).join(NODE.vendor_dir))?;
    //     }
    // }

    Ok(())
}

pub async fn prune() -> Result<(), AnyError> {
    let mut workspace = load_workspace_with_toolchain().await?;
    let manifest_path = workspace.root.join("dockerManifest.json");

    if !manifest_path.exists() {
        eprintln!("Unable to prune, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?");
        safe_exit(1);
    }

    let project_graph = generate_project_graph(&mut workspace)?;
    let manifest: DockerManifest = json::read(manifest_path)?;
    let mut is_using_node = false;

    for project_id in &manifest.focused_projects {
        let project = project_graph.get(project_id)?;

        // We use a match here to exhaustively check all languages
        match project.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
                is_using_node = true;
            }
            _ => {}
        }
    }

    // Only prune Node.js when one of the focused projects is Node.js based
    if is_using_node {
        prune_node(&workspace, &project_graph, &manifest).await?;
    }

    Ok(())
}
