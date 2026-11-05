use super::{DockerManifest, MANIFEST_NAME};
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{GlobPath, PortablePath};
use moon_layout_graph::{LayoutGraph, ToolchainDependenciesRoot};
use moon_pdk_api::{DefineDockerMetadataInput, ScaffoldDockerInput, ScaffoldDockerPhase};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_toolchain_plugin::ToolchainRegistry;
use rustc_hash::FxHashSet;
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument, trace, warn};

#[derive(Args, Clone, Debug)]
pub struct DockerScaffoldArgs {
    #[arg(required = true, help = "List of project IDs to copy sources for")]
    ids: Vec<Id>,
}

struct ScaffoldWorkflow {
    pub configs_root: PathBuf,
    pub sources_root: PathBuf,
    pub phase: ScaffoldDockerPhase,

    args: DockerScaffoldArgs,
    layout_graph: LayoutGraph,
    manifest: DockerManifest,
    project_graph: Arc<ProjectGraph>,
    registry: Arc<ToolchainRegistry>,
    session: MoonSession,
}

impl ScaffoldWorkflow {
    fn get_skeleton_root(&self) -> &Path {
        match self.phase {
            ScaffoldDockerPhase::Configs => &self.configs_root,
            ScaffoldDockerPhase::Sources => &self.sources_root,
        }
    }

    fn copy_files(&self, globs: Vec<String>, src: &Path, dst: &Path) -> miette::Result<()> {
        if !globs.is_empty() {
            for abs_file in glob::walk_files(src, &globs)? {
                fs::copy_file(&abs_file, dst.join(abs_file.strip_prefix(src).unwrap()))?;
            }
        }

        Ok(())
    }

    /// Globs from the `docker.scaffold` config for the current phase, from the
    /// project, otherwise the workspace. Includes ignore rules, as they travel
    /// with the skeleton.
    fn get_config_globs(&self, project: Option<&Project>) -> Vec<String> {
        let workspace_scaffold = &self.session.workspace_config.docker.scaffold;
        let project_scaffold = project.map(|p| &p.config.docker.scaffold);

        let mut globs = match self.phase {
            ScaffoldDockerPhase::Configs => {
                if let Some(scaffold) = project_scaffold
                    && !scaffold.configs_phase_globs.is_empty()
                {
                    scaffold.configs_phase_globs.clone()
                } else {
                    workspace_scaffold.configs_phase_globs.clone()
                }
            }
            ScaffoldDockerPhase::Sources => {
                let mut list = if let Some(scaffold) = project_scaffold
                    && !scaffold.sources_phase_globs.is_empty()
                {
                    scaffold.sources_phase_globs.clone()
                } else {
                    workspace_scaffold.sources_phase_globs.clone()
                };

                // Don't glob everything outside of a project (the workspace
                // root, or a dependencies root that encloses projects),
                // otherwise it will copy the entire repo!
                if list.is_empty() && project.is_some() {
                    list.push(GlobPath::parse("**/*").unwrap());
                }

                list
            }
        }
        .into_iter()
        .map(|glob| glob.to_string())
        .collect::<Vec<_>>();

        // Ignore rules travel with the skeleton. A build that keeps the git
        // directory reachable otherwise sees every file the package manager
        // installed as untracked, because `--exclude-standard` has no rules to
        // apply. This is matched per copied directory rather than recursively,
        // so each project contributes its own file without a full tree walk.
        globs.push(".gitignore".into());

        globs
    }

    async fn copy_files_from_plugins(
        &self,
        src: &Path,
        dst: &Path,
        project: Option<&Project>,
    ) -> miette::Result<()> {
        let outputs = self
            .registry
            .define_docker_metadata_all(|toolchain| DefineDockerMetadataInput {
                context: self.registry.create_context(),
                toolchain_config: match project {
                    Some(proj) => self
                        .registry
                        .create_merged_config(&toolchain.id, &proj.config),
                    None => self.registry.create_config(&toolchain.id),
                },
            })
            .await?;

        let mut globs =
            FxHashSet::from_iter(outputs.into_iter().flat_map(|output| output.scaffold_globs));

        globs.insert(format!(
            "moon.{}",
            self.session.config_loader.get_ext_glob()
        ));

        globs.extend(self.get_config_globs(project));

        self.copy_files(globs.into_iter().collect(), src, dst)?;

        Ok(())
    }

    /// Dependencies roots that aren't the workspace root or a project's root,
    /// like a workspace whose members are projects, but itself isn't one. Their
    /// files aren't copied otherwise, as they're outside of the project graph.
    /// When projects are provided, only the roots they're placed in. Roots of
    /// different toolchains at the same path are grouped.
    fn get_detached_roots(
        &self,
        project_ids: Option<&FxHashSet<Id>>,
    ) -> BTreeMap<&WorkspaceRelativePathBuf, Vec<&ToolchainDependenciesRoot>> {
        let mut groups = BTreeMap::<_, Vec<&ToolchainDependenciesRoot>>::default();

        let roots: Vec<&ToolchainDependenciesRoot> = match project_ids {
            Some(ids) => ids
                .iter()
                .flat_map(|id| self.layout_graph.bindings(id))
                .filter_map(|binding| binding.root_index())
                .map(|index| self.layout_graph.root(index))
                .collect(),
            None => self.layout_graph.roots().iter().collect(),
        };

        for root in roots {
            if root.root.as_str().is_empty() || root.owner_project.is_some() {
                continue;
            }

            let group = groups.entry(&root.root).or_default();

            if !group.iter().any(|other| other.toolchain == root.toolchain) {
                group.push(root);
            }
        }

        groups
    }

    /// Scaffold a dependencies root that isn't a project, like the workspace
    /// root: with the workspace's config, and without a project. Only the files
    /// of the toolchains that located it are copied.
    #[instrument(skip(self, roots))]
    async fn scaffold_dependencies_root(
        &self,
        root: &WorkspaceRelativePathBuf,
        roots: &[&ToolchainDependenciesRoot],
    ) -> miette::Result<()> {
        let input_root = root.to_logical_path(&self.session.workspace_root);
        let docker_root = root.to_logical_path(self.get_skeleton_root());

        fs::create_dir_all(&docker_root)?;

        debug!(
            scaffold_dir = ?docker_root,
            toolchains = ?roots.iter().map(|root| root.toolchain.as_str()).collect::<Vec<_>>(),
            "Copying files for dependencies root {}",
            color::file(root.as_str()),
        );

        let mut globs = FxHashSet::default();

        for deps_root in roots {
            let output = self
                .registry
                .load(&deps_root.toolchain)
                .await?
                .define_docker_metadata(DefineDockerMetadataInput {
                    context: self.registry.create_context(),
                    toolchain_config: deps_root.config.as_ref().to_owned(),
                })
                .await?;

            globs.extend(output.scaffold_globs);
        }

        globs.extend(self.get_config_globs(None));

        self.copy_files(globs.into_iter().collect(), &input_root, &docker_root)?;

        self.registry
            .scaffold_docker_many(
                roots.iter().map(|root| &root.toolchain).collect(),
                |toolchain| ScaffoldDockerInput {
                    context: self.registry.create_context(),
                    docker_config: self.session.workspace_config.docker.scaffold.clone(),
                    input_dir: toolchain.to_virtual_path(&input_root),
                    output_dir: toolchain.to_virtual_path(&docker_root),
                    phase: self.phase,
                    project: None,
                    toolchain_config: roots
                        .iter()
                        .find(|root| root.toolchain == toolchain.id)
                        .map(|root| root.config.as_ref().to_owned())
                        .unwrap_or_else(|| self.registry.create_config(&toolchain.id)),
                },
            )
            .await?;

        Ok(())
    }

    async fn scaffold_dependencies_roots(
        &self,
        project_ids: Option<&FxHashSet<Id>>,
    ) -> miette::Result<()> {
        for (root, roots) in self.get_detached_roots(project_ids) {
            self.scaffold_dependencies_root(root, &roots).await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn scaffold_project(&mut self, project: &Project) -> miette::Result<()> {
        let docker_project_root = project.source.to_logical_path(self.get_skeleton_root());
        let toolchains = project.get_enabled_toolchains();

        fs::create_dir_all(&docker_project_root)?;

        if self.phase == ScaffoldDockerPhase::Sources {
            debug!(
                scaffold_dir = ?docker_project_root,
                project_id = project.id.as_str(),
                toolchains = ?toolchains,
                "Copying sources for project {}",
                color::id(&project.id),
            );
        }

        self.copy_files_from_plugins(&project.root, &docker_project_root, Some(project))
            .await?;

        if !toolchains.is_empty() {
            self.registry
                .scaffold_docker_many(toolchains, |toolchain| ScaffoldDockerInput {
                    context: self.registry.create_context(),
                    docker_config: project.config.docker.scaffold.clone(),
                    input_dir: toolchain.to_virtual_path(&project.root),
                    output_dir: toolchain.to_virtual_path(&docker_project_root),
                    phase: self.phase,
                    project: Some(project.to_fragment()),
                    toolchain_config: self
                        .registry
                        .create_merged_config(&toolchain.id, &project.config),
                })
                .await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn scaffold_root(&mut self) -> miette::Result<()> {
        self.copy_files_from_plugins(&self.session.workspace_root, self.get_skeleton_root(), None)
            .await?;

        self.registry
            .scaffold_docker_many(self.registry.get_plugin_ids(), |toolchain| {
                ScaffoldDockerInput {
                    context: self.registry.create_context(),
                    docker_config: self.session.workspace_config.docker.scaffold.clone(),
                    input_dir: toolchain.to_virtual_path(&self.session.workspace_root),
                    output_dir: toolchain.to_virtual_path(self.get_skeleton_root()),
                    phase: self.phase,
                    project: None,
                    toolchain_config: self.registry.create_config(&toolchain.id),
                }
            })
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn scaffold_configs_skeleton(&mut self) -> miette::Result<()> {
        debug!(
            scaffold_dir = ?self.configs_root,
            "Scaffolding configs skeleton, copying configuration from all projects"
        );

        fs::create_dir_all(&self.configs_root)?;

        // Copy each project and mimic the folder structure
        for project in self.project_graph.get_all()? {
            self.scaffold_project(&project).await?;
        }

        // Then every dependencies root that isn't a project, as installing
        // a member requires its root's manifests and lockfiles
        self.scaffold_dependencies_roots(None).await?;

        self.scaffold_root().await?;

        // Copy moon configuration
        debug!(
            scaffold_dir = ?self.configs_root,
            "Copying moon configuration"
        );

        let cfg_dir_prefix = &self.session.config_loader.dir_prefix;
        let ext_glob = self.session.config_loader.get_ext_glob();

        self.copy_files(
            vec![
                format!("{cfg_dir_prefix}/*.{ext_glob}"),
                format!("{cfg_dir_prefix}/tasks/**/*.{ext_glob}"),
            ],
            &self.session.workspace_root,
            &self.configs_root,
        )?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn scaffold_sources_skeleton(&mut self) -> miette::Result<()> {
        self.manifest.focused_projects.extend(self.args.ids.clone());

        let mut queue = VecDeque::from(self.args.ids.clone());
        let mut visited = FxHashSet::default();
        let mut scaffolded = FxHashSet::default();

        debug!(
            scaffold_dir = ?self.sources_root,
            projects = ?self.args.ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            "Scaffolding sources skeleton, copying files from focused projects"
        );

        fs::create_dir_all(&self.sources_root)?;

        while let Some(project_id) = queue.pop_front() {
            if visited.contains(&project_id) {
                continue;
            }

            let project = self.project_graph.get(&project_id)?;

            for dep_config in &project.dependencies {
                // Avoid root-level projects as it will pull in all sources,
                // which is usually not what users want. If they do want it,
                // they can be explicit in config or on the command line!
                if !dep_config.is_root_scope() {
                    trace!(
                        project_id = project_id.as_str(),
                        dep_id = dep_config.id.as_str(),
                        "Including dependency project"
                    );

                    queue.push_back(dep_config.id.clone());
                }
            }

            self.scaffold_project(&project).await?;
            scaffolded.insert(project.id.clone());

            if !self.manifest.focused_projects.contains(&project_id) {
                self.manifest.unfocused_projects.insert(project_id.clone());
            }

            visited.insert(project_id);
        }

        // Like the workspace root, the roots of the copied projects
        // (by their IDs, as the arguments may be aliases)
        self.scaffold_dependencies_roots(Some(&scaffolded)).await?;

        self.scaffold_root().await?;

        Ok(())
    }

    fn sync_manifest(&self) -> miette::Result<()> {
        json::write_file(self.sources_root.join(MANIFEST_NAME), &self.manifest, true)?;
        json::write_file(self.configs_root.join(MANIFEST_NAME), &self.manifest, true)?;

        Ok(())
    }
}

fn check_docker_ignore(workspace_root: &Path) -> miette::Result<()> {
    let ignore_file = workspace_root.join(".dockerignore");
    let mut is_ignored = false;

    debug!(
        ignore_file = ?ignore_file,
        "Checking if moon cache has been ignored"
    );

    if ignore_file.exists() {
        let ignore = fs::read_file(&ignore_file)?;

        // Check lines so we can match exactly and avoid comments or nested paths
        for line in ignore.lines() {
            let line_clean = line
                .trim()
                .trim_start_matches("./")
                .trim_start_matches('/')
                .trim_end_matches('/');

            if line_clean == ".moon/cache" || line_clean == ".config/moon/cache" {
                is_ignored = true;
                break;
            }
        }
    }

    if !is_ignored {
        warn!(
            ignore_file = ?ignore_file,
            "{} must be ignored in {} to avoid interoperability issues when running {} commands inside and outside of Docker",
            color::file(".moon/cache"),
            color::file(".dockerignore"),
            color::shell("moon"),
        );

        warn!(
            "If you're not building from the workspace root, or are ignoring by other means, you can ignore this warning"
        );
    }

    Ok(())
}

#[instrument(skip(session))]
pub async fn scaffold(session: MoonSession, args: DockerScaffoldArgs) -> SessionResult {
    check_docker_ignore(&session.workspace_root)?;

    let docker_root = session.config_dir.join("docker");

    debug!(
        scaffold_root = ?docker_root,
        "Scaffolding monorepo structure to temporary docker directory",
    );

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root)?;
    fs::create_dir_all(&docker_root)?;

    // Place every project, as the configs skeleton has every
    // project, so it needs every dependencies root too
    let mut layout_builder = session.build_layout_graph().await?;

    layout_builder.place_all().await?;

    // Create the skeleton
    let mut workflow = ScaffoldWorkflow {
        configs_root: docker_root.join("configs"),
        sources_root: docker_root.join("sources"),
        phase: ScaffoldDockerPhase::Configs,
        args,
        layout_graph: layout_builder.build(),
        manifest: DockerManifest::default(),
        project_graph: session.get_project_graph().await?,
        registry: session.get_toolchain_registry().await?,
        session,
    };

    workflow.phase = ScaffoldDockerPhase::Configs;
    workflow.scaffold_configs_skeleton().await?;

    workflow.phase = ScaffoldDockerPhase::Sources;
    workflow.scaffold_sources_skeleton().await?;

    workflow.sync_manifest()?;

    Ok(None)
}
