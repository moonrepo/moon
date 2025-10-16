use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::MoonSession;
use moon_actions::plugins::{ExecCommandOptions, exec_plugin_command};
use moon_pdk_api::{InstallDependenciesInput, LocateDependenciesRootInput, PruneDockerInput};
use moon_project::Project;
use moon_toolchain_plugin::ToolchainPlugin;
use starbase::AppResult;
use starbase_utils::json;
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
                            .flat_map(|project| project.aliases.clone())
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
pub async fn prune(session: MoonSession) -> AppResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    let manifest: DockerManifest = json::read_file(manifest_path)?;

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Pruning dependencies for focused projects"
    );

    prune_toolchains(&session, &manifest).await?;

    Ok(None)
}
