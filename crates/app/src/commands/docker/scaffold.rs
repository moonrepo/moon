use super::{DockerManifest, MANIFEST_NAME};
use crate::session::MoonSession;
use async_recursion::async_recursion;
use clap::Args;
use moon_common::{Id, consts::*};
use moon_config::{GlobPath, PortablePath};
use moon_pdk_api::{DefineDockerMetadataInput, ScaffoldDockerInput, ScaffoldDockerPhase};
use moon_project::Project;
use moon_project_graph::{GraphConnections, ProjectGraph};
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::path::Path;
use tracing::{debug, instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct DockerScaffoldArgs {
    #[arg(required = true, help = "List of project IDs to copy sources for")]
    ids: Vec<Id>,
}

async fn gather_globs(
    session: &MoonSession,
    project: Option<&Project>,
    phase: ScaffoldDockerPhase,
) -> miette::Result<FxHashSet<String>> {
    let workspace_scaffold = &session.workspace_config.docker.scaffold;
    let project_scaffold = project.map(|p| &p.config.docker.scaffold);

    let outputs = session
        .get_toolchain_registry()
        .await?
        .define_docker_metadata_all(|registry, toolchain| DefineDockerMetadataInput {
            context: registry.create_context(),
            toolchain_config: match project {
                Some(proj) => registry.create_merged_config(&toolchain.id, &proj.config),
                None => registry.create_config(&toolchain.id),
            },
        })
        .await?;

    let mut globs =
        FxHashSet::from_iter(outputs.into_iter().flat_map(|output| output.scaffold_globs));

    globs.insert(format!("moon.{}", session.config_loader.get_ext_glob()));

    globs.extend(
        match phase {
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

    Ok(globs)
}

fn copy_files<I: IntoIterator<Item = String>>(
    globs: I,
    source: &Path,
    dest: &Path,
) -> miette::Result<()> {
    let globs = globs.into_iter().collect::<Vec<_>>();

    if !globs.is_empty() {
        for abs_file in glob::walk_files(source, &globs)? {
            fs::copy_file(&abs_file, dest.join(abs_file.strip_prefix(source).unwrap()))?;
        }
    }

    Ok(())
}

#[instrument(skip(session))]
async fn scaffold_root(
    session: &MoonSession,
    docker_root: &Path,
    phase: ScaffoldDockerPhase,
) -> miette::Result<()> {
    let toolchain_registry = session.get_toolchain_registry().await?;

    toolchain_registry
        .scaffold_docker_many(
            toolchain_registry.get_plugin_ids(),
            |registry, toolchain| ScaffoldDockerInput {
                context: registry.create_context(),
                docker_config: session.workspace_config.docker.scaffold.clone(),
                input_dir: toolchain.to_virtual_path(&session.workspace_root),
                output_dir: toolchain.to_virtual_path(docker_root),
                phase,
                project: None,
                toolchain_config: registry.create_config(&toolchain.id),
            },
        )
        .await?;

    copy_files(
        gather_globs(session, None, phase).await?,
        &session.workspace_root,
        docker_root,
    )?;

    Ok(())
}

#[instrument(skip(session))]
async fn scaffold_configs_project(
    session: &MoonSession,
    docker_configs_root: &Path,
    project: &Project,
) -> miette::Result<()> {
    let docker_project_root = project.source.to_logical_path(docker_configs_root);
    let toolchains = project.get_enabled_toolchains();

    if !toolchains.is_empty() {
        fs::create_dir_all(&docker_project_root)?;

        session
            .get_toolchain_registry()
            .await?
            .scaffold_docker_many(toolchains, |registry, toolchain| ScaffoldDockerInput {
                context: registry.create_context(),
                docker_config: session.workspace_config.docker.scaffold.clone(),
                input_dir: toolchain.to_virtual_path(&project.root),
                output_dir: toolchain.to_virtual_path(&docker_project_root),
                phase: ScaffoldDockerPhase::Configs,
                project: Some(project.to_fragment()),
                toolchain_config: registry.create_merged_config(&toolchain.id, &project.config),
            })
            .await?;
    }

    copy_files(
        gather_globs(session, Some(project), ScaffoldDockerPhase::Configs).await?,
        &project.root,
        &docker_project_root,
    )?;

    Ok(())
}

#[instrument(skip(session, project_graph))]
async fn scaffold_configs(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
) -> miette::Result<()> {
    let docker_configs_root = docker_root.join("configs");

    debug!(
        scaffold_dir = ?docker_configs_root,
        "Scaffolding configs skeleton, copying configuration from all projects"
    );

    fs::create_dir_all(&docker_configs_root)?;

    // Copy each project and mimic the folder structure
    for project in project_graph.get_all()? {
        scaffold_configs_project(session, &docker_configs_root, &project).await?;
    }

    scaffold_root(session, &docker_configs_root, ScaffoldDockerPhase::Configs).await?;

    // Copy moon configuration
    debug!(
        scaffold_dir = ?docker_configs_root,
        "Copying moon configuration"
    );

    let ext_glob = session.config_loader.get_ext_glob();

    copy_files(
        [
            format!(".moon/*.{ext_glob}"),
            format!(".moon/tasks/**/*.{ext_glob}"),
        ],
        &session.workspace_root,
        &docker_configs_root,
    )?;

    Ok(())
}

#[instrument(skip(session, project_graph, manifest, visited))]
#[async_recursion]
async fn scaffold_sources_project(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_sources_root: &Path,
    project_id: &Id,
    manifest: &mut DockerManifest,
    visited: &mut FxHashSet<Id>,
) -> miette::Result<()> {
    // Skip if already visited
    if visited.contains(project_id) {
        return Ok(());
    }

    visited.insert(project_id.to_owned());
    manifest.focused_projects.insert(project_id.to_owned());

    let project = project_graph.get(project_id)?;
    let toolchains = project.get_enabled_toolchains();
    let docker_project_root = project.source.to_logical_path(docker_sources_root);

    // Gather globs and copy
    debug!(
        scaffold_dir = ?docker_project_root,
        project_id = project_id.as_str(),
        toolchains = ?toolchains,
        "Copying sources for project {}",
        color::id(project_id),
    );

    copy_files(
        gather_globs(session, Some(&project), ScaffoldDockerPhase::Sources).await?,
        &project.root,
        &docker_project_root,
    )?;

    if !toolchains.is_empty() {
        session
            .get_toolchain_registry()
            .await?
            .scaffold_docker_many(toolchains, |registry, toolchain| ScaffoldDockerInput {
                context: registry.create_context(),
                docker_config: session.workspace_config.docker.scaffold.clone(),
                input_dir: toolchain.to_virtual_path(&project.root),
                output_dir: toolchain.to_virtual_path(&docker_project_root),
                phase: ScaffoldDockerPhase::Sources,
                project: Some(project.to_fragment()),
                toolchain_config: registry.create_merged_config(&toolchain.id, &project.config),
            })
            .await?;
    }

    for dep_config in &project.dependencies {
        // Avoid root-level projects as it will pull in all sources,
        // which is usually not what users want. If they do want it,
        // they can be explicit in config or on the command line!
        if !dep_config.is_root_scope() {
            debug!(
                project_id = project_id.as_str(),
                dep_id = dep_config.id.as_str(),
                "Including dependency project"
            );

            scaffold_sources_project(
                session,
                project_graph,
                docker_sources_root,
                &dep_config.id,
                manifest,
                visited,
            )
            .await?;
        }
    }

    Ok(())
}

#[instrument(skip(session, project_graph))]
async fn scaffold_sources(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
    project_ids: &[Id],
) -> miette::Result<()> {
    let docker_sources_root = docker_root.join("sources");

    debug!(
        scaffold_dir = ?docker_sources_root,
        projects = ?project_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Scaffolding sources skeleton, copying files from focused projects"
    );

    let mut manifest = DockerManifest {
        focused_projects: FxHashSet::default(),
        unfocused_projects: FxHashSet::default(),
    };

    let mut visited = FxHashSet::default();

    // Copy all focused projects
    for project_id in project_ids {
        scaffold_sources_project(
            session,
            project_graph,
            &docker_sources_root,
            project_id,
            &mut manifest,
            &mut visited,
        )
        .await?;
    }

    scaffold_root(session, &docker_sources_root, ScaffoldDockerPhase::Sources).await?;

    // Include non-focused projects in the manifest
    for project_id in project_graph.get_node_keys() {
        if !manifest.focused_projects.contains(&project_id) {
            manifest.unfocused_projects.insert(project_id);
        }
    }

    json::write_file(docker_sources_root.join(MANIFEST_NAME), &manifest, true)?;

    // Sync to the workspace scaffold for staged builds
    json::write_file(
        docker_root.join("configs").join(MANIFEST_NAME),
        &manifest,
        true,
    )?;

    Ok(())
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
            if line
                .trim()
                .trim_start_matches("./")
                .trim_start_matches('/')
                .trim_end_matches('/')
                == ".moon/cache"
            {
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

#[instrument(skip_all)]
pub async fn scaffold(session: MoonSession, args: DockerScaffoldArgs) -> AppResult {
    check_docker_ignore(&session.workspace_root)?;

    let docker_root = session.workspace_root.join(CONFIG_DIRNAME).join("docker");

    debug!(
        scaffold_root = ?docker_root,
        "Scaffolding monorepo structure to temporary docker directory",
    );

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root)?;
    fs::create_dir_all(&docker_root)?;

    // Create the skeleton
    let project_graph = session.get_project_graph().await?;

    scaffold_configs(&session, &project_graph, &docker_root).await?;

    scaffold_sources(&session, &project_graph, &docker_root, &args.ids).await?;

    Ok(None)
}
