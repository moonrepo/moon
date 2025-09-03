use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::MoonSession;
use moon_actions::plugins::{ExecCommandOptions, exec_plugin_command};
use moon_bun_tool::BunTool;
use moon_common::Id;
use moon_config::PlatformType;
use moon_deno_tool::DenoTool;
use moon_node_lang::PackageJsonCache;
use moon_node_tool::NodeTool;
use moon_pdk_api::{InstallDependenciesInput, LocateDependenciesRootInput, PruneDockerInput};
use moon_platform::PlatformManager;
use moon_project::Project;
use moon_rust_tool::RustTool;
use moon_tool::DependencyManager;
use moon_toolchain_plugin::ToolchainPlugin;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_utils::{fs, json};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

struct PruneToolchainInstance {
    deps_root: PathBuf,
    projects: Vec<Arc<Project>>,
    toolchain: Arc<ToolchainPlugin>,
}

#[instrument(skip_all)]
pub async fn prune_toolchains(session: &MoonSession, manifest: &DockerManifest) -> AppResult {
    let project_graph = session.get_project_graph().await?;
    let toolchain_registry = session.get_toolchain_registry().await?;

    // Collect all dependency roots and which projects belong to it
    let mut deps_roots: Vec<PruneToolchainInstance> = vec![];

    for project_id in &manifest.focused_projects {
        let project = project_graph.get(project_id)?;

        for locate_result in toolchain_registry
            .locate_dependencies_root_many(
                project.get_enabled_toolchains(),
                |registry, toolchain| LocateDependenciesRootInput {
                    context: registry.create_context(),
                    starting_dir: toolchain.to_virtual_path(&project.root),
                    toolchain_config: registry.create_merged_config(
                        &toolchain.id,
                        &session.toolchain_config,
                        &project.config,
                    ),
                },
            )
            .await?
        {
            if let Some(root) = locate_result.output.root.as_ref() {
                let toolchain = locate_result.toolchain;

                if !toolchain.in_dependencies_workspace(&locate_result.output, &project.root)? {
                    continue;
                }

                match deps_roots.iter_mut().find(|instance| {
                    &instance.deps_root == root && instance.toolchain.id == toolchain.id
                }) {
                    Some(entry) => {
                        entry.projects.push(project.clone());
                    }
                    None => {
                        deps_roots.push(PruneToolchainInstance {
                            deps_root: root.into(),
                            projects: vec![project.clone()],
                            toolchain,
                        });
                    }
                };
            }
        }
    }

    if deps_roots.is_empty() {
        return Ok(None);
    }

    // Then prune and install dependencies for each root (and its projects)
    let mut set = JoinSet::new();

    for instance in deps_roots {
        let toolchain_registry = Arc::clone(&toolchain_registry);
        let toolchain = Arc::clone(&instance.toolchain);
        let docker_config = session.workspace_config.docker.prune.clone();
        let app_context = session.get_app_context().await?;

        set.spawn(async move {
            // Run prune first, so this can remove all development artifacts
            if toolchain.has_func("prune_docker").await {
                let _output = toolchain
                    .prune_docker(PruneDockerInput {
                        context: toolchain_registry.create_context(),
                        docker_config: docker_config.clone(),
                        projects: instance
                            .projects
                            .iter()
                            .map(|project| project.to_fragment())
                            .collect(),
                        root: toolchain.to_virtual_path(&instance.deps_root),
                        toolchain_config: toolchain_registry
                            .create_config(&toolchain.id, &app_context.toolchain_config),
                    })
                    .await?;
            }

            // Then run install, so this can only install production dependencies
            if toolchain.has_func("install_dependencies").await
                && docker_config.install_toolchain_deps
            {
                let in_project = if instance.projects.len() == 1
                    && instance
                        .projects
                        .first()
                        .is_some_and(|project| project.root == instance.deps_root)
                {
                    instance.projects.first().cloned()
                } else {
                    None
                };

                let output = toolchain
                    .install_dependencies(InstallDependenciesInput {
                        context: toolchain_registry.create_context(),
                        packages: instance
                            .projects
                            .iter()
                            .flat_map(|project| project.alias.clone())
                            .collect(),
                        production: true,
                        project: in_project.as_ref().map(|project| project.to_fragment()),
                        root: toolchain.to_virtual_path(&instance.deps_root),
                        toolchain_config: match &in_project {
                            Some(project) => toolchain_registry.create_merged_config(
                                &toolchain.id,
                                &app_context.toolchain_config,
                                &project.config,
                            ),
                            None => toolchain_registry
                                .create_config(&toolchain.id, &app_context.toolchain_config),
                        },
                    })
                    .await?;

                if let Some(mut install) = output.install_command {
                    // Always execute without cache
                    install.cache = None;

                    // Always stream output to the console
                    install.command.stream = true;

                    exec_plugin_command(
                        app_context,
                        &install,
                        &ExecCommandOptions {
                            project: in_project,
                            prefix: "prune-docker".into(),
                            working_dir: Some(instance.deps_root),
                            on_exec: None,
                        },
                    )
                    .await?;
                }
            }

            Ok::<_, miette::Report>(())
        });
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_bun(
    bun: &BunTool,
    session: &MoonSession,
    manifest: &DockerManifest,
) -> AppResult {
    let project_graph = session.get_project_graph().await?;

    // Some package managers do not delete stale node modules
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        debug!("Removing Bun vendor directories (node_modules)");

        fs::remove_dir_all(session.workspace_root.join("node_modules"))?;

        for source in project_graph.sources().values() {
            fs::remove_dir_all(source.join("node_modules").to_path(&session.workspace_root))?;
        }
    }

    // Install production only dependencies for focused projects
    if session.workspace_config.docker.prune.install_toolchain_deps {
        let mut package_names = vec![];

        for project_id in &manifest.focused_projects {
            if let Some(source) = project_graph.sources().get(project_id)
                && let Some(package_json) =
                    PackageJsonCache::read(source.to_path(&session.workspace_root))?
                && let Some(package_name) = package_json.data.name
            {
                package_names.push(package_name);
            }
        }

        debug!(
            packages = ?package_names,
            "Pruning Bun dependencies"
        );

        bun.install_focused_dependencies(&(), &package_names, true)
            .await?;
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_deno(
    deno: &DenoTool,
    session: &MoonSession,
    _manifest: &DockerManifest,
) -> AppResult {
    // noop
    if session.workspace_config.docker.prune.install_toolchain_deps {
        deno.install_focused_dependencies(&(), &[], true).await?;
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune_node(
    node: &NodeTool,
    session: &MoonSession,
    manifest: &DockerManifest,
) -> AppResult {
    let project_graph = session.get_project_graph().await?;

    // Some package managers do not delete stale node modules
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        debug!("Removing Node.js vendor directories (node_modules)");

        fs::remove_dir_all(session.workspace_root.join("node_modules"))?;

        for source in project_graph.sources().values() {
            fs::remove_dir_all(source.join("node_modules").to_path(&session.workspace_root))?;
        }
    }

    // Install production only dependencies for focused projects
    if session.workspace_config.docker.prune.install_toolchain_deps {
        let mut package_names = vec![];

        for project_id in &manifest.focused_projects {
            if let Some(source) = project_graph.sources().get(project_id)
                && let Some(package_json) =
                    PackageJsonCache::read(source.to_path(&session.workspace_root))?
                && let Some(package_name) = package_json.data.name
            {
                package_names.push(package_name);
            }
        }

        debug!(
            packages = ?package_names,
            "Pruning Node.js dependencies"
        );

        node.get_package_manager()
            .install_focused_dependencies(node, &package_names, true)
            .await?;
    }

    Ok(None)
}

// This assumes that the project was built in --release mode. Is this correct?
#[instrument(skip_all)]
pub async fn prune_rust(_rust: &RustTool, session: &MoonSession) -> AppResult {
    if session
        .workspace_config
        .docker
        .prune
        .delete_vendor_directories
    {
        let target_dir = &session.workspace_root.join("target");
        let lockfile_path = &session.workspace_root.join("Cargo.lock");

        // Only delete target if relative to `Cargo.lock`
        if target_dir.exists() && lockfile_path.exists() {
            debug!(
                target_dir = ?target_dir,
                "Deleting Rust target directory"
            );

            fs::remove_dir_all(target_dir)?;
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn prune(session: MoonSession) -> AppResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    let workspace_graph = session.get_workspace_graph().await?;
    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let mut toolchains = FxHashSet::<Id>::default();

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Pruning dependencies for focused projects"
    );

    for project_id in &manifest.focused_projects {
        toolchains.extend(
            workspace_graph
                .get_project(project_id)?
                .get_enabled_toolchains()
                .into_iter()
                .cloned(),
        );
    }

    // Do this later so we only run once for each platform instead of per project
    for toolchain_id in toolchains {
        if toolchain_id == "unknown" {
            // Will crash with "Platform unknown has not been enabled"
            continue;
        }

        if let Ok(platform) = PlatformManager::write().get_by_toolchain_mut(&toolchain_id) {
            platform.setup_toolchain().await?;

            match platform.get_type() {
                PlatformType::Bun => {
                    prune_bun(
                        platform
                            .get_tool()?
                            .as_any()
                            .downcast_ref::<BunTool>()
                            .unwrap(),
                        &session,
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
                        &session,
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
                        &session,
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
                        &session,
                    )
                    .await?;
                }
                _ => {}
            };
        }
    }

    prune_toolchains(&session, &manifest).await?;

    Ok(None)
}
