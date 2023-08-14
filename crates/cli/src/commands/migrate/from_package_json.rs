use super::check_dirty_repo;
use moon::{generate_project_graph, load_workspace};
use moon_common::consts::CONFIG_PROJECT_FILENAME;
use moon_common::Id;
use moon_config::{
    DependencyScope, PartialDependencyConfig, PartialProjectDependsOn, PartialTaskEntry,
    ProjectConfig,
};
use moon_logger::info;
use moon_node_lang::package_json::{DepsSet, PackageJson};
use moon_node_platform::create_tasks_from_scripts;
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_utils::yaml;
use std::collections::BTreeMap;

const LOG_TARGET: &str = "moon:migrate:from-package-json";

pub async fn from_package_json(project_id: Id, skip_touched_files_check: bool) -> AppResult {
    let mut workspace = load_workspace().await?;

    if skip_touched_files_check {
        info!(target: LOG_TARGET, "Skipping touched files check.");
    } else {
        check_dirty_repo(&workspace).await?;
    };

    // Create a mapping of `package.json` names to project IDs
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut package_map: FxHashMap<String, Id> = FxHashMap::default();

    for project in project_graph.get_all_unexpanded() {
        if let Some(package_json) = PackageJson::read(&project.root)? {
            if let Some(package_name) = package_json.name {
                package_map.insert(package_name, project.id.to_owned());
            }
        }
    }

    // Create or update the local `moon.yml`
    let project = project_graph.get(&project_id)?;
    let mut partial_config = ProjectConfig::load_partial(&project.root)?;

    let mut link_deps = |deps: &DepsSet, scope: DependencyScope| {
        for package_name in deps.keys() {
            if let Some(dep_id) = package_map.get(package_name) {
                partial_config.depends_on.get_or_insert(vec![]).push(
                    if matches!(scope, DependencyScope::Production) {
                        PartialProjectDependsOn::String(dep_id.to_owned())
                    } else {
                        PartialProjectDependsOn::Object(PartialDependencyConfig {
                            id: Some(dep_id.to_owned()),
                            scope: Some(scope),
                            ..PartialDependencyConfig::default()
                        })
                    },
                );
            }
        }
    };

    PackageJson::sync(&project.root, |package_json| {
        // Create tasks from `package.json` scripts
        for (task_id, task_config) in create_tasks_from_scripts(&project.id, package_json)? {
            partial_config
                .tasks
                .get_or_insert(BTreeMap::new())
                .insert(task_id, PartialTaskEntry::Base(task_config));
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

    yaml::write_with_config(project.root.join(CONFIG_PROJECT_FILENAME), &partial_config)?;

    Ok(())
}
