use super::{DockerManifest, MANIFEST_NAME};
use crate::session::CliSession;
use async_recursion::async_recursion;
use clap::Args;
use moon_common::consts::*;
use moon_common::{path, Id};
use moon_config::LanguageType;
use moon_project_graph::ProjectGraph;
use moon_rust_lang::cargo_toml::{CargoTomlCache, CargoTomlExt};
use moon_toolchain::detect::detect_language_files;
use rustc_hash::FxHashSet;
use schematic::ConfigEnum;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{fs, glob, json};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, warn};

#[derive(Args, Clone, Debug)]
pub struct DockerScaffoldArgs {
    #[arg(required = true, help = "List of project IDs to copy sources for")]
    ids: Vec<Id>,

    #[arg(long, help = "Additional file globs to include in sources")]
    include: Vec<String>,
}

fn copy_files_from_paths(paths: Vec<PathBuf>, source: &Path, dest: &Path) -> AppResult {
    let mut files = vec![];

    for file in paths {
        files.push(path::to_string(file.strip_prefix(source).unwrap())?);
    }

    copy_files(&files, source, dest)
}

fn copy_files<T: AsRef<str>>(list: &[T], source: &Path, dest: &Path) -> AppResult {
    for file in list {
        let file = file.as_ref();
        let source_file = source.join(file);

        if file != "." && source_file.exists() {
            if source_file.is_dir() {
                fs::copy_dir_all(&source_file, &source_file, &dest.join(file))?;
            } else {
                fs::copy_file(source_file, dest.join(file))?;
            }
        }
    }

    Ok(())
}

fn create_files<T: AsRef<str>>(list: &[T], dest: &Path) -> AppResult {
    for file in list {
        let file = file.as_ref();
        let dest_file = dest.join(file);

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

#[instrument(skip(session, project_graph))]
async fn scaffold_workspace(
    session: &CliSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
) -> AppResult {
    let docker_workspace_root = docker_root.join("workspace");
    let projects = project_graph.get_all()?;

    debug!(
        scaffold_dir = ?docker_workspace_root,
        "Scaffolding workspace skeleton, including configuration from all projects"
    );

    fs::create_dir_all(&docker_workspace_root)?;

    // Copy manifest and config files for every type of language,
    // not just the one the project is configured as!
    let copy_from_dir = |source: &Path, dest: &Path, project_lang: LanguageType| -> AppResult {
        let mut files_to_copy: Vec<String> = vec![
            ".gitignore".into(),
            ".prototools".into(),
            CONFIG_PROJECT_FILENAME_YML.into(),
            CONFIG_PROJECT_FILENAME_PKL.into(),
            CONFIG_TEMPLATE_FILENAME_YML.into(),
            CONFIG_TEMPLATE_FILENAME_PKL.into(),
        ];
        let mut files_to_create: Vec<String> = vec![];

        if session
            .workspace_config
            .docker
            .scaffold
            .copy_toolchain_files
        {
            for lang in LanguageType::variants() {
                files_to_copy.extend(detect_language_files(&lang));

                // These are special cases
                match lang {
                    LanguageType::JavaScript => {
                        files_to_copy.extend([
                            "postinstall.js".into(),
                            "postinstall.cjs".into(),
                            "postinstall.mjs".into(),
                        ]);
                    }
                    LanguageType::Rust => {
                        if let Some(cargo_toml) = CargoTomlCache::read(source)? {
                            let manifests = cargo_toml.get_member_manifest_paths(source)?;

                            // Non-workspace
                            if manifests.is_empty() {
                                if lang == project_lang {
                                    files_to_create
                                        .extend(["src/lib.rs".into(), "src/main.rs".into()]);
                                }
                            }
                            // Workspace
                            else {
                                for manifest in manifests {
                                    if let Ok(rel_manifest) = manifest.strip_prefix(source) {
                                        files_to_copy.push(path::to_string(rel_manifest)?);

                                        let rel_manifest_dir = rel_manifest.parent().unwrap();

                                        if lang == project_lang {
                                            files_to_create.extend([
                                                path::to_string(
                                                    rel_manifest_dir.join("src/lib.rs"),
                                                )?,
                                                path::to_string(
                                                    rel_manifest_dir.join("src/main.rs"),
                                                )?,
                                            ]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    LanguageType::TypeScript => {
                        if let Some(typescript_config) = &session.toolchain_config.typescript {
                            files_to_copy
                                .push(typescript_config.project_config_file_name.to_owned());
                            files_to_copy.push(typescript_config.root_config_file_name.to_owned());
                            files_to_copy
                                .push(typescript_config.root_options_config_file_name.to_owned());
                        }
                    }
                    _ => {}
                }
            }
        }

        copy_files(&files_to_copy, source, dest)?;
        create_files(&files_to_create, dest)?;

        Ok(())
    };

    // Copy each project and mimic the folder structure
    let mut has_root_project = false;

    for project in projects {
        if project.source.as_str() == "." {
            has_root_project = true;
        }

        let docker_project_root = docker_workspace_root.join(project.source.as_str());

        fs::create_dir_all(&docker_project_root)?;

        copy_from_dir(
            &project.root,
            &docker_project_root,
            project.language.clone(),
        )?;
    }

    // Copy root lockfiles and configurations
    if !has_root_project {
        copy_from_dir(
            &session.workspace_root,
            &docker_workspace_root,
            LanguageType::Unknown,
        )?;
    }

    if session
        .workspace_config
        .docker
        .scaffold
        .copy_toolchain_files
    {
        if let Some(js_config) = &session.toolchain_config.bun {
            if js_config.packages_root != "." {
                copy_from_dir(
                    &session.workspace_root.join(&js_config.packages_root),
                    &docker_workspace_root.join(&js_config.packages_root),
                    LanguageType::Unknown,
                )?;
            }
        }

        if let Some(js_config) = &session.toolchain_config.node {
            if js_config.packages_root != "." {
                copy_from_dir(
                    &session.workspace_root.join(&js_config.packages_root),
                    &docker_workspace_root.join(&js_config.packages_root),
                    LanguageType::Unknown,
                )?;
            }
        }

        if let Some(typescript_config) = &session.toolchain_config.typescript {
            if typescript_config.root != "." {
                copy_from_dir(
                    &session.workspace_root.join(&typescript_config.root),
                    &docker_workspace_root.join(&typescript_config.root),
                    LanguageType::Unknown,
                )?;
            }
        }
    }

    // Copy moon configuration
    let moon_dir = session.workspace_root.join(CONFIG_DIRNAME);

    debug!(
        scaffold_dir = ?docker_workspace_root,
        moon_dir = ?moon_dir,
        "Copying core moon configuration"
    );

    copy_files_from_paths(
        glob::walk_files(
            moon_dir,
            ["*.pkl", "tasks/**/*.pkl", "*.yml", "tasks/**/*.yml"],
        )?,
        &session.workspace_root,
        &docker_workspace_root,
    )?;

    // Include via globs
    let include = &session.workspace_config.docker.scaffold.include;

    if !include.is_empty() {
        debug!(
            include = ?include,
            "Including additional files"
        );

        copy_files_from_paths(
            glob::walk_files(&session.workspace_root, include)?,
            &session.workspace_root,
            &docker_workspace_root,
        )?;
    }

    Ok(())
}

#[instrument(skip(session, project_graph, manifest))]
#[async_recursion]
async fn scaffold_sources_project(
    session: &CliSession,
    project_graph: &ProjectGraph,
    docker_sources_root: &Path,
    project_id: &Id,
    manifest: &mut DockerManifest,
) -> AppResult {
    let project = project_graph.get(project_id)?;
    let mut include_globs = vec!["!node_modules/**", "!target/**/*", "!vendor/**"];

    manifest.focused_projects.insert(project_id.to_owned());

    if project.config.docker.scaffold.include.is_empty() {
        include_globs.push("**/*");
    } else {
        include_globs.extend(
            project
                .config
                .docker
                .scaffold
                .include
                .iter()
                .map(|glob| glob.as_str()),
        );
    }

    debug!(
        id = project_id.as_str(),
        globs = ?include_globs,
        "Copying sources from project {}",
        color::id(project_id),
    );

    copy_files_from_paths(
        glob::walk_files(&project.root, include_globs)?,
        &session.workspace_root,
        docker_sources_root,
    )?;

    for dep_cfg in &project.dependencies {
        // Avoid root-level projects as it will pull in all sources,
        // which is usually not what users want. If they do want it,
        // they can be explicit in config or on the command line!
        if !dep_cfg.is_root_scope() {
            debug!(
                id = project_id.as_str(),
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

    Ok(())
}

#[instrument(skip(session, project_graph))]
async fn scaffold_sources(
    session: &CliSession,
    project_graph: &ProjectGraph,
    docker_root: &Path,
    project_ids: &[Id],
    include: &[String],
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
    for project_id in project_graph.ids() {
        if !manifest.focused_projects.contains(project_id) {
            manifest.unfocused_projects.insert(project_id.to_owned());
        }
    }

    // Include via globs
    if !include.is_empty() {
        warn!(
            "The --include argument is deprecated, use the {} settings instead",
            color::property("docker")
        );

        debug!(
            include = ?include,
            "Including additional sources"
        );

        copy_files_from_paths(
            glob::walk_files(&session.workspace_root, include)?,
            &session.workspace_root,
            &docker_sources_root,
        )?;
    }

    json::write_file(docker_sources_root.join(MANIFEST_NAME), &manifest, true)?;

    // Sync to the workspace scaffold for staged builds
    json::write_file(
        docker_root.join("workspace").join(MANIFEST_NAME),
        &manifest,
        true,
    )?;

    Ok(())
}

pub fn check_docker_ignore(workspace_root: &Path) -> miette::Result<()> {
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
pub async fn scaffold(session: CliSession, args: DockerScaffoldArgs) -> AppResult {
    check_docker_ignore(&session.workspace_root)?;

    let docker_root = session.workspace_root.join(CONFIG_DIRNAME).join("docker");

    debug!(
        scaffold_root = ?docker_root,
        "Scaffolding monorepo structure to temporary docker directory",
    );

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root)?;
    fs::create_dir_all(&docker_root)?;

    // Create the workspace skeleton
    let project_graph = session.get_project_graph().await?;

    scaffold_workspace(&session, &project_graph, &docker_root).await?;

    scaffold_sources(
        &session,
        &project_graph,
        &docker_root,
        &args.ids,
        &args.include,
    )
    .await?;

    Ok(())
}
