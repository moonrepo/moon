use super::check_dirty_repo;
use crate::session::CliSession;
use clap::Args;
use moon_common::consts::CONFIG_PROJECT_FILENAME_YML;
use moon_common::Id;
use moon_config::{
    DependencyScope, NodePackageManager, PartialDependencyConfig, PartialProjectDependsOn,
    ProjectConfig,
};
use moon_node_lang::package_json::DependenciesMap;
use moon_node_lang::PackageJsonCache;
use moon_node_platform::create_tasks_from_scripts;
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_utils::yaml;
use std::collections::BTreeMap;
use tracing::{info, instrument};

#[derive(Args, Clone, Debug)]
pub struct FromPackageJsonArgs {
    #[arg(help = "ID of project to migrate")]
    id: Id,

    #[arg(long, hide = true)]
    pub skip_touched_files_check: bool,
}

#[instrument(skip_all)]
pub async fn from_package_json(session: CliSession, args: FromPackageJsonArgs) -> AppResult {
    if args.skip_touched_files_check {
        info!("Skipping touched files check.");
    } else {
        check_dirty_repo(&session).await?;
    };

    // Create a mapping of `package.json` names to project IDs
    let project_graph = session.get_project_graph().await?;
    let mut package_map: FxHashMap<String, Id> = FxHashMap::default();

    for project in project_graph.get_all_unexpanded() {
        if let Some(package_json) = PackageJsonCache::read(&project.root)? {
            if let Some(package_name) = package_json.data.name {
                package_map.insert(package_name, project.id.to_owned());
            }
        }
    }

    // Create or update the local `moon.*`
    let project = project_graph.get(&args.id)?;
    let mut partial_config = ProjectConfig::load_partial(&project.root)?;

    let mut link_deps = |deps: &DependenciesMap<String>, scope: DependencyScope| {
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

    PackageJsonCache::sync(&project.root, |package_json| {
        // Create tasks from `package.json` scripts
        for (task_id, task_config) in create_tasks_from_scripts(
            &project.id,
            package_json,
            session
                .toolchain_config
                .node
                .as_ref()
                .map(|cfg| cfg.package_manager)
                .unwrap_or(NodePackageManager::Npm),
        )? {
            partial_config
                .tasks
                .get_or_insert(BTreeMap::new())
                .insert(task_id, task_config);
        }

        // Link deps from `package.json` dependencies
        if let Some(deps) = &package_json.data.dependencies {
            link_deps(deps, DependencyScope::Production);
        }

        if let Some(deps) = &package_json.data.dev_dependencies {
            link_deps(deps, DependencyScope::Development);
        }

        if let Some(deps) = &package_json.data.peer_dependencies {
            link_deps(deps, DependencyScope::Peer);
        }

        Ok(true)
    })?;

    yaml::write_file_with_config(
        project.root.join(CONFIG_PROJECT_FILENAME_YML),
        &partial_config,
    )?;

    Ok(())
}
