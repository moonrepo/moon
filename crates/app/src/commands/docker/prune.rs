use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::session::{MoonSession, SessionResult};
use miette::IntoDiagnostic;
use moon_actions::plugins::{ExecCommandOptions, exec_plugin_command};
use moon_layout_graph::{RootIndex, RootKind, ToolchainDependenciesRoot};
use moon_pdk_api::{InstallDependenciesInput, PruneDockerInput};
use moon_project::Project;
use moon_toolchain::DependenciesWorkspace;
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashMap;
use starbase_utils::json;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

/// A dependencies root that focused projects were placed in.
struct PruneRoot {
    /// Unfocused projects in this root, that the focused projects depend on.
    dependencies: Vec<Arc<Project>>,

    /// The owner project of a package root, which its install is for.
    owner: Option<Arc<Project>>,

    /// The focused projects in this root.
    projects: Vec<Arc<Project>>,

    root: ToolchainDependenciesRoot,
    toolchain: Arc<ToolchainPlugin>,

    /// The root and its member globs, for matching unfocused projects.
    workspace: DependenciesWorkspace,
}

struct PruneWorkflow {
    manifest: DockerManifest,
    registry: Arc<ToolchainRegistry>,
    roots: Vec<PruneRoot>,
    session: MoonSession,
    workspace_graph: Arc<WorkspaceGraph>,
}

impl PruneWorkflow {
    async fn prune(mut self) -> miette::Result<()> {
        self.gather_roots().await?;

        if self.roots.is_empty() {
            debug!("No dependencies roots for focused projects, skipping prune");

            return Ok(());
        }

        self.prune_dependencies().await?;

        Ok(())
    }

    async fn gather_roots(&mut self) -> miette::Result<()> {
        debug!(
            project_ids = ?self.manifest.focused_projects,
            "Placing focused projects into dependencies roots",
        );

        let focused = self
            .manifest
            .focused_projects
            .iter()
            .map(|project_id| self.workspace_graph.get_project(project_id))
            .collect::<miette::Result<Vec<_>>>()?;

        // Only the focused projects' own toolchains are placed, like the
        // ones that `docker setup` installs dependencies for
        // (collected first, as the iterator's borrows would make the future not `Send`)
        let pairs = focused
            .iter()
            .flat_map(|project| {
                project
                    .get_enabled_toolchains()
                    .into_iter()
                    .map(|toolchain_id| (Arc::clone(project), toolchain_id.to_owned()))
            })
            .collect::<Vec<_>>();

        let mut builder = self.session.build_layout_graph().await?;

        builder.place(pairs).await?;

        let layout_graph = builder.build();
        let mut indices = FxHashMap::<RootIndex, usize>::default();

        for project in &focused {
            for binding in layout_graph.bindings(&project.id) {
                let Some(index) = binding.root_index() else {
                    debug!(
                        project_id = project.id.as_str(),
                        toolchain_id = binding.toolchain.as_str(),
                        "Not in a dependencies root, skipping!",
                    );

                    continue;
                };

                let position = match indices.get(&index) {
                    Some(position) => *position,
                    None => {
                        let root = layout_graph.root(index).to_owned();

                        self.roots.push(PruneRoot {
                            dependencies: vec![],
                            owner: match (&root.kind, &root.owner_project) {
                                (RootKind::Package, Some(owner_id)) => {
                                    Some(self.workspace_graph.get_project(owner_id)?)
                                }
                                _ => None,
                            },
                            projects: vec![],
                            toolchain: self.registry.load(&root.toolchain).await?,
                            workspace: DependenciesWorkspace {
                                root: root.root.to_logical_path(&self.session.workspace_root),
                                members: match &root.kind {
                                    RootKind::Workspace { members } => Some(members.to_owned()),
                                    RootKind::Package => None,
                                },
                            },
                            root,
                        });

                        indices.insert(index, self.roots.len() - 1);
                        self.roots.len() - 1
                    }
                };

                debug!(
                    project_id = project.id.as_str(),
                    deps_root = self.roots[position].root.root.as_str(),
                    "Adding to dependencies root",
                );

                self.roots[position].projects.push(Arc::clone(project));
            }
        }

        // Also include dependencies (unfocused) in the roots, so we can prune
        // them too. They're not placed, but matched against each root's members
        for project_id in &self.manifest.unfocused_projects {
            let project = self.workspace_graph.get_project(project_id)?;
            let toolchains = project.get_enabled_toolchains();

            for prune_root in &mut self.roots {
                if toolchains.contains(&&prune_root.root.toolchain)
                    && prune_root
                        .toolchain
                        .in_dependencies_workspace(&prune_root.workspace, &project.root)?
                        .is_some()
                    && !prune_root.dependencies.contains(&project)
                {
                    prune_root.dependencies.push(Arc::clone(&project));
                }
            }
        }

        Ok(())
    }

    async fn prune_dependencies(self) -> miette::Result<()> {
        let mut set = JoinSet::new();

        for prune_root in self.roots {
            let toolchain_registry = Arc::clone(&self.registry);
            let toolchain = Arc::clone(&prune_root.toolchain);
            let docker_config = self.session.workspace_config.docker.prune.clone();
            let app_context = self.session.get_app_context().await?;

            set.spawn(Box::pin(async move {
                let root = &prune_root.workspace.root;

                // Run prune first, so this can remove all development artifacts.
                // A root's actions use its config: its owner project's (a package,
                // or a workspace's root project), otherwise the workspace's
                toolchain
                    .prune_docker(PruneDockerInput {
                        context: toolchain_registry.create_context(),
                        docker_config: docker_config.clone(),
                        project_dependencies: prune_root
                            .dependencies
                            .iter()
                            .map(|project| project.to_fragment())
                            .collect(),
                        projects: prune_root
                            .projects
                            .iter()
                            .map(|project| project.to_fragment())
                            .collect(),
                        root: toolchain.to_virtual_path(root),
                        toolchain_config: prune_root.root.config.as_ref().to_owned(),
                    })
                    .await?;

                // Then run install, so this can only install production dependencies
                if docker_config.install_toolchain_dependencies {
                    let output = toolchain
                        .install_dependencies(InstallDependenciesInput {
                            context: toolchain_registry.create_context(),
                            packages: get_install_packages_for_projects(
                                &prune_root.projects,
                                &toolchain.id,
                            ),
                            production: true,
                            project: prune_root
                                .owner
                                .as_ref()
                                .map(|project| project.to_fragment()),
                            root: toolchain.to_virtual_path(root),
                            toolchain_config: prune_root.root.config.as_ref().to_owned(),
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
                                project: prune_root.owner,
                                prefix: "prune-docker".into(),
                                working_dir: Some(prune_root.workspace.root),
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
        registry: session.get_toolchain_registry().await?,
        roots: vec![],
        workspace_graph: session.get_workspace_graph().await?,
        session,
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
