use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_lang_node::package::{DepsSet, PackageJson};
use moon_plugin_node::create_tasks_from_scripts;
use moon_utils::fs;
use moon_workspace::Workspace;
use serde_yaml::to_string;
use std::collections::HashMap;

pub async fn from_package_json(project_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Create a mapping of `package.json` names to project IDs
    let mut package_map: HashMap<String, String> = HashMap::new();

    for id in workspace.projects.ids() {
        let project = workspace.projects.load(&id)?;

        if let Some(package_json) = PackageJson::read(&project.root).await? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, id);
            }
        }
    }

    // Create or update the local `project.yml`
    let mut project = workspace.projects.load(project_id)?;

    let mut link_deps = |deps: &DepsSet| {
        for package_name in deps.keys() {
            if let Some(dep_id) = package_map.get(package_name) {
                project.config.depends_on.push(dep_id.to_owned());
            }
        }
    };

    PackageJson::sync(&project.root, |package_json| {
        // Create tasks from `package.json` scripts
        for (task_id, task) in create_tasks_from_scripts(project_id, package_json).unwrap() {
            project.config.tasks.insert(task_id, task.to_config());
        }

        // Link deps from `package.json` dependencies
        if let Some(deps) = &package_json.dependencies {
            link_deps(deps);
        }

        if let Some(deps) = &package_json.dev_dependencies {
            link_deps(deps);
        }

        if let Some(deps) = &package_json.peer_dependencies {
            link_deps(deps);
        }

        Ok(())
    })
    .await?;

    fs::write(
        project.root.join(CONFIG_PROJECT_FILENAME),
        to_string(&project.config)?,
    )
    .await?;

    Ok(())
}
