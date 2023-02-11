use super::check_dirty_repo;
use crate::helpers::AnyError;
use moon::{generate_project_graph, load_workspace};
use moon_config::{DependencyConfig, DependencyScope, ProjectDependsOn};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_error::MoonError;
use moon_logger::info;
use moon_node_lang::package::{DepsSet, PackageJson};
use moon_node_platform::create_tasks_from_scripts;
use moon_utils::yaml;
use rustc_hash::FxHashMap;

pub async fn from_package_json(
    project_id: String,
    skip_touched_files_check: bool,
) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;

    if skip_touched_files_check {
        info!("Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };

    // Create a mapping of `package.json` names to project IDs
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut package_map: FxHashMap<String, String> = FxHashMap::default();

    for project in project_graph.get_all()? {
        if let Some(package_json) = PackageJson::read(&project.root)? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, project.id.to_owned());
            }
        }
    }

    // Create or update the local `moon.yml`
    let mut project = project_graph.get(&project_id)?.to_owned();

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

        Ok(true)
    })?;

    yaml::write_with_config(project.root.join(CONFIG_PROJECT_FILENAME), &project.config)?;

    Ok(())
}
