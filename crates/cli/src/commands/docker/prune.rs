use crate::commands::docker::scaffold::DockerManifest;
use crate::helpers::load_workspace;
use futures::future::try_join_all;
use moon_lang_node::{package::PackageJson, NODE};
use moon_terminal::safe_exit;
use moon_utils::fs;

pub async fn prune() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let toolchain = &workspace.toolchain;
    let manifest_path = workspace.root.join("dockerManifest.json");

    if !manifest_path.exists() {
        eprintln!("Unable to prune, docker manifest missing. Has it been scaffolded?");
        safe_exit(1);
    }

    let manifest: DockerManifest = fs::read_json(manifest_path).await?;
    let mut futures = vec![];

    if workspace.config.node.is_some() {
        let mut package_names = vec![];

        for project_id in &manifest.focused_projects {
            let project = workspace.projects.load(project_id)?;

            if let Some(package_json) = PackageJson::read(&project.root)? {
                if let Some(package_name) = &package_json.name {
                    package_names.push(package_name.clone());
                }
            }
        }

        // Install production only dependencies
        toolchain
            .get_node()?
            .get_package_manager()
            .install_focused_dependencies(toolchain, &package_names, true)
            .await?;

        // Remove extraneous node module folders
        for (project_id, project_source) in &workspace.projects.projects_map {
            if manifest.unfocused_projects.contains(project_id) {
                futures.push(fs::remove_dir_all(
                    workspace.root.join(project_source).join(NODE.vendor_dir),
                ));
            }
        }
    }

    try_join_all(futures).await?;

    Ok(())
}
