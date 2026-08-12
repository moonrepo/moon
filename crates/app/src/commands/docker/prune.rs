use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::{MoonSession, SessionResult};
use miette::IntoDiagnostic;
use moon_actions::plugins::{ExecCommandOptions, exec_plugin_command};
use moon_pdk_api::{InstallDependenciesInput, LocateDependenciesRootInput, PruneDockerInput};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_toolchain::DependenciesWorkspace;
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use starbase_utils::json;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

#[derive(Debug)]
struct ToolchainDependenciesWorkspace {
    dependencies: Vec<Arc<Project>>,
    output: DependenciesWorkspace,
    projects: Vec<Arc<Project>>,
    toolchain: Arc<ToolchainPlugin>,
}

struct PruneWorkflow {
    manifest: DockerManifest,
    project_graph: Arc<ProjectGraph>,
    registry: Arc<ToolchainRegistry>,
    session: MoonSession,
    workspaces: Vec<ToolchainDependenciesWorkspace>,
}

impl PruneWorkflow {
    async fn prune(mut self) -> miette::Result<()> {
        self.gather_workspaces().await?;

        if self.workspaces.is_empty() {
            debug!("No dependency workspaces for focused projects, skipping prune");

            return Ok(());
        }

        self.prune_dependencies().await?;

        Ok(())
    }

    async fn gather_workspaces(&mut self) -> miette::Result<()> {
        debug!(
            project_ids = ?self.manifest.focused_projects,
            "Locating dependency workspaces for focused projects",
        );

        for project_id in &self.manifest.focused_projects {
            let project = self.project_graph.get(project_id)?;

            for locate_result in self
                .registry
                .locate_dependencies_root_many(project.get_enabled_toolchains(), |toolchain| {
                    LocateDependenciesRootInput {
                        context: self.registry.create_context(),
                        starting_dir: toolchain.to_virtual_path(&project.root),
                        toolchain_config: self
                            .registry
                            .create_merged_config(&toolchain.id, &project.config),
                    }
                })
                .await?
            {
                let toolchain = locate_result.plugin;
                let output = locate_result.output;

                if !toolchain.in_dependencies_workspace(&output, &project.root)? {
                    debug!(
                        project_id = project.id.as_str(),
                        project_root = ?project.root,
                        deps_root = ?output.root,
                        "Not in a dependency workspace, skipping!",
                    );

                    continue;
                }

                debug!(
                    project_id = project.id.as_str(),
                    project_root = ?project.root,
                    deps_root = ?output.root,
                    "Adding to dependency workspace",
                );

                match self.workspaces.iter_mut().find(|workspace| {
                    workspace.output.root == output.root && workspace.toolchain.id == toolchain.id
                }) {
                    Some(workspace) => {
                        workspace.projects.push(project.clone());
                    }
                    None => {
                        self.workspaces.push(ToolchainDependenciesWorkspace {
                            dependencies: vec![],
                            output,
                            projects: vec![project.clone()],
                            toolchain,
                        });
                    }
                };
            }
        }

        // Also include dependencies (unfocused) in the workspace so we can prune them too
        for project_id in &self.manifest.unfocused_projects {
            let project = self.project_graph.get(project_id)?;
            let toolchains = project.get_enabled_toolchains();

            for workspace in &mut self.workspaces {
                if toolchains.iter().any(|id| *id == &workspace.toolchain.id)
                    && workspace
                        .toolchain
                        .in_dependencies_workspace(&workspace.output, &project.root)?
                    && !workspace.dependencies.contains(&project)
                {
                    workspace.dependencies.push(project.clone());
                }
            }
        }

        Ok(())
    }

    async fn prune_dependencies(self) -> miette::Result<()> {
        let mut set = JoinSet::new();

        for workspace in self.workspaces {
            let toolchain_registry = Arc::clone(&self.registry);
            let toolchain = Arc::clone(&workspace.toolchain);
            let docker_config = self.session.workspace_config.docker.prune.clone();
            let app_context = self.session.get_app_context().await?;

            set.spawn(Box::pin(async move {
                // Run prune first, so this can remove all development artifacts
                toolchain
                    .prune_docker(PruneDockerInput {
                        context: toolchain_registry.create_context(),
                        docker_config: docker_config.clone(),
                        project_dependencies: workspace
                            .dependencies
                            .iter()
                            .map(|project| project.to_fragment())
                            .collect(),
                        projects: workspace
                            .projects
                            .iter()
                            .map(|project| project.to_fragment())
                            .collect(),
                        root: toolchain.to_virtual_path(&workspace.output.root),
                        toolchain_config: toolchain_registry.create_config(&toolchain.id),
                    })
                    .await?;

                // Then run install, so this can only install production dependencies
                if docker_config.install_toolchain_dependencies {
                    let in_project = if workspace.projects.len() == 1
                        && workspace
                            .projects
                            .first()
                            .is_some_and(|project| project.root == workspace.output.root)
                    {
                        workspace.projects.first().cloned()
                    } else {
                        None
                    };

                    let output = toolchain
                        .install_dependencies(InstallDependenciesInput {
                            context: toolchain_registry.create_context(),
                            packages: get_install_packages_for_projects(
                                &workspace.projects,
                                &toolchain.id,
                            ),
                            production: true,
                            project: in_project.as_ref().map(|project| project.to_fragment()),
                            root: toolchain.to_virtual_path(&workspace.output.root),
                            toolchain_config: match &in_project {
                                Some(project) => toolchain_registry
                                    .create_merged_config(&toolchain.id, &project.config),
                                None => toolchain_registry.create_config(&toolchain.id),
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
                                working_dir: Some(workspace.output.root),
                                on_exec: None,
                            },
                        )
                        .await?;
                    }
                }

                Ok::<_, miette::Report>(())
            }));
        }

        while let Some(result) = set.join_next().await {
            result.into_diagnostic()??;
        }

        Ok(())
    }
}

fn get_install_packages_for_projects(projects: &[Arc<Project>], toolchain_id: &str) -> Vec<String> {
    projects
        .iter()
        .flat_map(|project| {
            project.aliases.iter().filter_map(|alias| {
                if alias.plugin == toolchain_id {
                    Some(alias.alias.clone())
                } else {
                    None
                }
            })
        })
        .collect()
}

#[instrument(skip_all)]
pub async fn prune(session: MoonSession) -> SessionResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    let manifest: DockerManifest = json::read_file(manifest_path)?;

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Pruning dependencies for focused projects"
    );

    let workflow = PruneWorkflow {
        manifest,
        project_graph: session.get_project_graph().await?,
        registry: session.get_toolchain_registry().await?,
        session,
        workspaces: vec![],
    };

    workflow.prune().await?;

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_common::Id;
    use moon_project::ProjectAlias;

    fn create_project(id: &str, aliases: Vec<ProjectAlias>) -> Arc<Project> {
        Arc::new(Project {
            id: Id::raw(id),
            aliases,
            ..Project::default()
        })
    }

    #[test]
    fn uses_matching_toolchain_aliases_as_install_packages() {
        let projects = vec![create_project(
            "app",
            vec![
                ProjectAlias {
                    alias: "@scope/app".into(),
                    plugin: Id::raw("javascript"),
                },
                ProjectAlias {
                    alias: "py-app".into(),
                    plugin: Id::raw("python"),
                },
            ],
        )];

        assert_eq!(
            get_install_packages_for_projects(&projects, "javascript"),
            ["@scope/app"]
        );
    }

    #[test]
    fn uses_same_id_aliases_as_install_packages() {
        let projects = vec![create_project(
            "app",
            vec![ProjectAlias {
                alias: "app".into(),
                plugin: Id::raw("javascript"),
            }],
        )];

        assert_eq!(
            get_install_packages_for_projects(&projects, "javascript"),
            ["app"]
        );
    }

    #[test]
    fn preserves_empty_packages_without_aliases() {
        let projects = vec![create_project("app", vec![])];

        assert_eq!(
            get_install_packages_for_projects(&projects, "javascript"),
            Vec::<String>::new()
        );
    }
}
