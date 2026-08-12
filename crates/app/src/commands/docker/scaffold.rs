use super::{DockerManifest, MANIFEST_NAME};
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_common::Id;
use moon_config::{GlobPath, PortablePath};
use moon_pdk_api::{DefineDockerMetadataInput, ScaffoldDockerInput, ScaffoldDockerPhase};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_toolchain_plugin::ToolchainRegistry;
use rustc_hash::FxHashSet;
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::collections::VecDeque;
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

    async fn copy_files_from_plugins(
        &self,
        src: &Path,
        dst: &Path,
        project: Option<&Project>,
    ) -> miette::Result<()> {
        let workspace_scaffold = &self.session.workspace_config.docker.scaffold;
        let project_scaffold = project.map(|p| &p.config.docker.scaffold);

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

        globs.extend(
            match self.phase {
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

                    // Don't glob everything at the workspace root,
                    // otherwise it will copy the entire repo!
                    if list.is_empty() && project.is_some() {
                        list.push(GlobPath::parse("**/*").unwrap());
                    }

                    list
                }
            }
            .into_iter()
            .map(|glob| glob.to_string()),
        );

        self.copy_files(globs.into_iter().collect(), src, dst)?;

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

            if !self.manifest.focused_projects.contains(&project_id) {
                self.manifest.unfocused_projects.insert(project_id.clone());
            }

            visited.insert(project_id);
        }

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

    // Create the skeleton
    let mut workflow = ScaffoldWorkflow {
        configs_root: docker_root.join("configs"),
        sources_root: docker_root.join("sources"),
        phase: ScaffoldDockerPhase::Configs,
        args,
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
