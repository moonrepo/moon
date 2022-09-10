use crate::helpers::load_workspace;
use moon_config::{DependencyConfig, DependencyScope, ProjectDependsOn};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_error::MoonError;
use moon_lang_node::package::{DepsSet, PackageJson};
use moon_platform_node::create_tasks_from_scripts;
use moon_utils::fs;
use serde_yaml::to_string;
use std::collections::HashMap;

pub async fn from_package_json(project_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;

    // Create a mapping of `package.json` names to project IDs
    let mut package_map: HashMap<String, String> = HashMap::new();

    for id in workspace.projects.ids() {
        let project = workspace.projects.load(&id)?;

        if let Some(package_json) = PackageJson::read(&project.root)? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, id);
            }
        }
    }

    // Create or update the local `moon.yml`
    let mut project = workspace.projects.load(project_id)?;

    let mut link_deps = |deps: &DepsSet, scope: DependencyScope| {
        for package_name in deps.keys() {
            if let Some(dep_id) = package_map.get(package_name) {
                project
                    .config
                    .depends_on
                    .push(if matches!(scope, DependencyScope::Production) {
                        ProjectDependsOn::String(dep_id.to_owned())
                    } else {
                        ProjectDependsOn::Object(DependencyConfig {
                            id: dep_id.to_owned(),
                            scope: scope.clone(),
                            via: None,
                        })
                    });
            }
        }
    };

    PackageJson::sync(&project.root, |package_json| {
        // Create tasks from `package.json` scripts
        for (task_id, task_config) in create_tasks_from_scripts(&project.id, package_json)
            .map_err(|e| MoonError::Generic(e.to_string()))?
        {
            project.config.tasks.insert(task_id, task_config);
        }

        // Link deps from `package.json` dependencies
        if let Some(deps) = &package_json.dependencies {
            link_deps(deps, DependencyScope::Production);
        }

        if let Some(deps) = &package_json.dev_dependencies {
            link_deps(deps, DependencyScope::Development);
        }

        if let Some(deps) = &package_json.peer_dependencies {
            link_deps(deps, DependencyScope::Peer);
        }

        Ok(())
    })?;

    fs::write(
        project.root.join(CONFIG_PROJECT_FILENAME),
        to_string(&project.config)?,
    )
    .await?;

    Ok(())
}
