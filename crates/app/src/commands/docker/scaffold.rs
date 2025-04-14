use super::{DockerManifest, MANIFEST_NAME};
use crate::session::MoonSession;
use async_recursion::async_recursion;
use clap::Args;
use moon_common::consts::*;
use moon_common::{Id, path};
use moon_config::LanguageType;
use moon_pdk_api::{DefineDockerMetadataInput, ScaffoldDockerInput, ScaffoldDockerPhase};
use moon_project::Project;
use moon_project_graph::{GraphConnections, ProjectGraph};
use moon_rust_lang::cargo_toml::{CargoTomlCache, CargoTomlExt};
use moon_toolchain::detect::detect_language_files;
use rustc_hash::FxHashSet;
use schematic::ConfigEnum;
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
                Some(proj) => registry.create_merged_config(
                    &toolchain.id,
                    &session.toolchain_config,
                    &proj.config,
                ),
                None => registry.create_config(&toolchain.id, &session.toolchain_config),
            },
        })
        .await?;

    Ok(FxHashSet::from_iter(
        outputs.into_iter().flat_map(|output| output.scaffold_globs),
    ))
}

fn copy_files<F: IntoIterator<Item = String>, G: IntoIterator<Item = String>>(
    files: F,
    globs: G,
    source: &Path,
    dest: &Path,
) -> miette::Result<()> {
    let files = files.into_iter().collect::<Vec<_>>();
    let globs = globs.into_iter().collect::<Vec<_>>();

    if !files.is_empty() {
        for file in files {
            let abs_file = source.join(&file);

            if file != "." && abs_file.exists() {
                if abs_file.is_dir() {
                    fs::copy_dir_all(&abs_file, &abs_file, dest.join(file))?;
                } else {
                    fs::copy_file(abs_file, dest.join(file))?;
                }
            }
        }
    }

    if !globs.is_empty() {
        for abs_file in glob::walk_files(source, &globs)? {
            fs::copy_file(&abs_file, dest.join(abs_file.strip_prefix(source).unwrap()))?;
        }
    }

    Ok(())
}

fn create_files<I: IntoIterator<Item = String>>(list: I, dest: &Path) -> miette::Result<()> {
    for file in list {
        let dest_file = dest.join(&file);

        if dest_file.exists() {
            continue;
        }

        let mut data = "";

        if file.ends_with(".json") {
            data = "{}";
        }

        fs::write_file(dest.join(file), data.as_bytes())?;
    }

    Ok(())
}

fn scaffold_files(
    session: &MoonSession,
    src_dir: &Path,
    out_dir: &Path,
    shared_globs: &FxHashSet<String>,
    language: &LanguageType,
) -> AppResult {
    let mut files_to_create: FxHashSet<String> = FxHashSet::default();
    let mut files_to_copy: FxHashSet<String> =
        FxHashSet::from_iter([".gitignore".into(), ".prototools".into()]);
    let mut files_to_glob: FxHashSet<String> = FxHashSet::default();
    files_to_copy.extend(session.config_loader.get_project_file_names());
    files_to_copy.extend(session.config_loader.get_template_file_names());

    if session
        .workspace_config
        .docker
        .scaffold
        .copy_toolchain_files
    {
        files_to_glob.extend(shared_globs.to_owned());

        // Copy manifest and config files for every type of language,
        // not just the one the project is configured as!
        for lang in LanguageType::variants() {
            files_to_copy.extend(detect_language_files(&lang));

            // These are special cases
            match lang {
                LanguageType::JavaScript => {
                    files_to_glob.insert("postinstall.*".into());
                }
                LanguageType::Rust => {
                    if let Some(cargo_toml) = CargoTomlCache::read(src_dir)? {
                        let manifests = cargo_toml.get_member_manifest_paths(src_dir)?;

                        // Non-workspace
                        if manifests.is_empty() {
                            if &lang == language {
                                files_to_create.extend(["src/lib.rs".into(), "src/main.rs".into()]);
                            }
                        }
                        // Workspace
                        else {
                            for manifest in manifests {
                                if let Ok(rel_manifest) = manifest.strip_prefix(src_dir) {
                                    files_to_copy.insert(path::to_string(rel_manifest)?);

                                    let rel_manifest_dir = rel_manifest.parent().unwrap();

                                    if &lang == language {
                                        files_to_create.extend([
                                            path::to_string(rel_manifest_dir.join("src/lib.rs"))?,
                                            path::to_string(rel_manifest_dir.join("src/main.rs"))?,
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
                LanguageType::TypeScript => {
                    files_to_copy.insert("tsconfig.json".into());
                    files_to_copy.insert("tsconfig.options.json".into());
                }
                _ => {}
            }
        }
    }

    copy_files(files_to_copy, files_to_glob, src_dir, out_dir)?;
    create_files(files_to_create, out_dir)?;

    Ok(None)
}

#[instrument(skip(session))]
async fn scaffold_workspace_project(
    session: &MoonSession,
    docker_workspace_root: &Path,
    project: &Project,
    shared_globs: &FxHashSet<String>,
) -> AppResult {
    let docker_project_root = project.source.to_logical_path(docker_workspace_root);

    fs::create_dir_all(&docker_project_root)?;

    scaffold_files(
        session,
        &project.root,
        &docker_project_root,
        shared_globs,
        &project.language,
    )?;

    let toolchains = project.get_enabled_toolchains();

    if !toolchains.is_empty() {
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
            })
            .await?;
    }

    Ok(None)
}

#[instrument(skip(session, project_graph))]
async fn scaffold_workspace(
    session: &MoonSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
) -> AppResult {
    let docker_workspace_root = docker_root.join("workspace");
    let projects = project_graph.get_all()?;
    let shared_globs = get_toolchain_globs(session, None).await?;

    debug!(
        scaffold_dir = ?docker_workspace_root,
        "Scaffolding workspace skeleton, including configuration from all projects"
    );

    fs::create_dir_all(&docker_workspace_root)?;

    // Copy each project and mimic the folder structure
    let mut has_root_project = false;

    for project in projects {
        if path::is_root_level_source(&project.source) {
            has_root_project = true;
        }

        scaffold_workspace_project(session, &docker_workspace_root, &project, &shared_globs)
            .await?;
    }

    // Copy root lockfiles and configurations
    if !has_root_project {
        scaffold_files(
            session,
            &session.workspace_root,
            &docker_workspace_root,
            &shared_globs,
            &LanguageType::Unknown,
        )?;
    }

    // Copy moon configuration
    debug!(
        scaffold_dir = ?docker_workspace_root,
        "Copying core moon configuration"
    );

    copy_files(
        [],
        [
            ".moon/*.{pkl,yml}".to_owned(),
            ".moon/tasks/**/*.{pkl,yml}".to_owned(),
        ],
        &session.workspace_root,
        &docker_workspace_root,
    )?;

    // Include via globs
    let include = session
        .workspace_config
        .docker
        .scaffold
        .include
        .iter()
        .map(|glob| glob.to_string())
        .collect::<Vec<_>>();

    if !include.is_empty() {
        debug!(
            include = ?include,
            "Including additional files"
        );

        copy_files([], include, &session.workspace_root, &docker_workspace_root)?;
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
) -> AppResult {
    manifest.focused_projects.insert(project_id.to_owned());

    let project = project_graph.get(project_id)?;
    let toolchains = project.get_enabled_toolchains();

    // Gather globs
    let mut globs = get_toolchain_globs(session, Some(&project)).await?;

    globs.extend([
        "!node_modules/**/*".into(),
        "!target/**/*".into(),
        "!vendor/**/*".into(),
    ]);

    if project.config.docker.scaffold.include.is_empty() {
        globs.insert("**/*".into());
    } else {
        globs.extend(
            project
                .config
                .docker
                .scaffold
                .include
                .iter()
                .map(|glob| glob.to_string())
                .collect::<Vec<_>>(),
        );
    }

    debug!(
        project_id = project_id.as_str(),
        globs = ?globs,
        toolchains = ?toolchains,
        "Copying sources for project {}",
        color::id(project_id),
    );

    // Copy files
    let docker_project_root = project.source.to_logical_path(docker_sources_root);

    copy_files([], globs, &project.root, &docker_project_root)?;

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
            })
            .await?;
    }

    for dep_cfg in &project.dependencies {
        // Avoid root-level projects as it will pull in all sources,
        // which is usually not what users want. If they do want it,
        // they can be explicit in config or on the command line!
        if !dep_cfg.is_root_scope() {
            debug!(
                project_id = project_id.as_str(),
                dep_id = dep_cfg.id.as_str(),
                "Including dependency project"
            );

            scaffold_sources_project(
                session,
                project_graph,
                docker_sources_root,
                &dep_cfg.id,
                manifest,
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
        "Scaffolding sources skeleton, including files from focused projects"
    );

    let mut manifest = DockerManifest {
        focused_projects: FxHashSet::default(),
        unfocused_projects: FxHashSet::default(),
    };

    // Copy all projects
    for project_id in project_ids {
        scaffold_sources_project(
            session,
            project_graph,
            &docker_sources_root,
            project_id,
            &mut manifest,
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
        docker_root.join("workspace").join(MANIFEST_NAME),
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

    scaffold_workspace(&session, &project_graph, &docker_root).await?;

    scaffold_sources(&session, &project_graph, &docker_root, &args.ids).await?;

    Ok(None)
}
