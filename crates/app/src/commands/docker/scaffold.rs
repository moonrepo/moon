use super::{DockerManifest, MANIFEST_NAME};
use crate::session::MoonSession;
use async_recursion::async_recursion;
use clap::Args;
use moon_common::{Id, consts::*, path};
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

async fn get_toolchain_globs(
    session: &MoonSession,
    project: Option<&Project>,
) -> miette::Result<FxHashSet<String>> {
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

    Ok(FxHashSet::from_iter(
        outputs.into_iter().flat_map(|output| output.scaffold_globs),
    ))
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
async fn scaffold_configs_root(session: &MoonSession, docker_configs_root: &Path) -> AppResult {
    let toolchain_registry = session.get_toolchain_registry().await?;
    let shared_globs = get_toolchain_globs(session, None).await?;

    toolchain_registry
        .scaffold_docker_many(
            toolchain_registry.get_plugin_ids(),
            |registry, toolchain| ScaffoldDockerInput {
                context: registry.create_context(),
                docker_config: session.workspace_config.docker.scaffold.clone(),
                input_dir: toolchain.to_virtual_path(&session.workspace_root),
                output_dir: toolchain.to_virtual_path(docker_configs_root),
                phase: ScaffoldDockerPhase::Configs,
                project: None,
                toolchain_config: registry.create_config(&toolchain.id),
            },
        )
        .await?;

    if !shared_globs.is_empty() {
        copy_files(shared_globs, &session.workspace_root, docker_configs_root)?;
    }

    Ok(None)
}

#[instrument(skip(session))]
async fn scaffold_configs_project(
    session: &MoonSession,
    docker_configs_root: &Path,
    project: &Project,
) -> AppResult {
    let docker_project_root = project.source.to_logical_path(docker_configs_root);
    let shared_globs = get_toolchain_globs(session, Some(project)).await?;
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

    if !shared_globs.is_empty() {
        copy_files(shared_globs, &project.root, &docker_project_root)?;
    }

    Ok(None)
}

#[instrument(skip(session, project_graph))]
async fn scaffold_configs(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
) -> AppResult {
    let docker_configs_root = docker_root.join("configs");
    let projects = project_graph.get_all()?;

    debug!(
        scaffold_dir = ?docker_configs_root,
        "Scaffolding configs skeleton, copying configuration from all projects"
    );

    fs::create_dir_all(&docker_configs_root)?;

    // Copy each project and mimic the folder structure
    let mut has_root_project = false;

    for project in projects {
        if path::is_root_level_source(&project.source) {
            has_root_project = true;
        }

        scaffold_configs_project(session, &docker_configs_root, &project).await?;
    }

    if !has_root_project {
        scaffold_configs_root(session, &docker_configs_root).await?;
    }

    // Copy moon configuration
    debug!(
        scaffold_dir = ?docker_configs_root,
        "Copying core moon configuration"
    );

    copy_files(
        [
            ".moon/*.{pkl,yml}".to_owned(),
            ".moon/tasks/**/*.{pkl,yml}".to_owned(),
        ],
        &session.workspace_root,
        &docker_configs_root,
    )?;

    // Include via globs
    let globs = session
        .workspace_config
        .docker
        .scaffold
        .include
        .iter()
        .map(|glob| glob.to_string())
        .collect::<Vec<_>>();

    if !globs.is_empty() {
        debug!(
            scaffold_dir = ?docker_configs_root,
            globs = ?globs,
            "Copying additional files based on {}",
            color::property("docker.scaffold.include")
        );

        copy_files(globs, &session.workspace_root, &docker_configs_root)?;
    }

    Ok(None)
}

#[instrument(skip(session, project_graph, manifest))]
#[async_recursion]
async fn scaffold_sources_project(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_sources_root: &Path,
    project_id: &Id,
    manifest: &mut DockerManifest,
    visited: &mut FxHashSet<Id>,
) -> AppResult {
    // Skip if already visited
    if visited.contains(project_id) {
        return Ok(None);
    }

    visited.insert(project_id.to_owned());
    manifest.focused_projects.insert(project_id.to_owned());

    let project = project_graph.get(project_id)?;
    let toolchains = project.get_enabled_toolchains();
    let docker_project_root = project.source.to_logical_path(docker_sources_root);

    // Gather globs and copy
    let mut globs = project
        .config
        .docker
        .scaffold
        .include
        .iter()
        .map(|glob| glob.to_string())
        .collect::<Vec<_>>();

    if globs.is_empty() {
        globs.push("**/*".into());
    }

    debug!(
        scaffold_dir = ?docker_project_root,
        project_id = project_id.as_str(),
        globs = ?globs,
        toolchains = ?toolchains,
        "Copying sources for project {}",
        color::id(project_id),
    );

    copy_files(globs, &project.root, &docker_project_root)?;

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

    Ok(None)
}

#[instrument(skip(session, project_graph))]
async fn scaffold_sources(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
    project_ids: &[Id],
) -> AppResult {
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

    // Copy all projects
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

    Ok(None)
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
