use crate::commands::docker::scaffold::DockerManifest;
use crate::helpers::load_workspace;
use futures::future::try_join_all;
use moon_config::ProjectLanguage;
use moon_lang_node::{package::PackageJson, NODE};
use moon_terminal::safe_exit;
use moon_utils::fs;
use moon_workspace::Workspace;

pub async fn prune_node(
    workspace: &Workspace,
    manifest: &DockerManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let toolchain = &workspace.toolchain;
    let mut package_names = vec![];

    for project_id in &manifest.focused_projects {
        if let Some(project_source) = workspace.projects.projects_map.get(project_id) {
            if let Some(package_json) = PackageJson::read(workspace.root.join(project_source))? {
                if let Some(package_name) = package_json.name {
                    package_names.push(package_name);
                }
            }
        }
    }

    // Some package managers do not delete stale node modules
    let mut futures = vec![fs::remove_dir_all(workspace.root.join(NODE.vendor_dir))];

    for project_source in workspace.projects.projects_map.values() {
        futures.push(fs::remove_dir_all(
            workspace.root.join(project_source).join(NODE.vendor_dir),
        ));
    }

    try_join_all(futures).await?;

    // Install production only dependencies for focused projects
    toolchain
        .get_node()?
        .get_package_manager()
        .install_focused_dependencies(toolchain, &package_names, true)
        .await?;

    // Remove extraneous node module folders for unfocused projects
    let mut futures = vec![];

    for project_id in &manifest.unfocused_projects {
        if let Some(project_source) = workspace.projects.projects_map.get(project_id) {
            futures.push(fs::remove_dir_all(
                workspace.root.join(project_source).join(NODE.vendor_dir),
            ));
        }
    }

    try_join_all(futures).await?;

    Ok(())
}

pub async fn prune() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let manifest_path = workspace.root.join("dockerManifest.json");

    if !manifest_path.exists() {
        eprintln!("Unable to prune, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?");
        safe_exit(1);
    }

    let manifest: DockerManifest = fs::read_json(manifest_path).await?;
    let mut is_using_node = false;

    for project_id in &manifest.focused_projects {
        let project = workspace.projects.load(project_id)?;

        // We use a match here to exhaustively check all languages
        match project.config.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
                is_using_node = true;
            }
            _ => {}
        }
    }

    // Only prune Node.js when one of the focused projects is Node.js based
    if is_using_node {
        prune_node(&workspace, &manifest).await?;
    }

    Ok(())
}
