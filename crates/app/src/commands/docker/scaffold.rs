use super::{DockerManifest, MANIFEST_NAME};
use crate::session::CliSession;
use async_recursion::async_recursion;
use clap::Args;
use moon_common::consts::{CONFIG_DIRNAME, CONFIG_PROJECT_FILENAME, CONFIG_TEMPLATE_FILENAME};
use moon_common::{path, Id};
use moon_config::LanguageType;
use moon_platform_detector::detect_language_files;
use moon_rust_lang::cargo_toml::{CargoTomlCache, CargoTomlExt};
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

    #[arg(long, help = "Additional file globs to include in sources")]
    include: Vec<String>,
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

#[instrument(skip(session))]
async fn scaffold_workspace(session: &CliSession, docker_root: &Path) -> AppResult {
    let docker_workspace_root = docker_root.join("workspace");
    let projects = session.get_project_graph().await?.get_all()?;

    debug!(
        scaffold_dir = ?docker_workspace_root,
        "Scaffolding workspace skeleton, including configuration from all projects"
    );

    fs::create_dir_all(&docker_workspace_root)?;

    // Copy manifest and config files for every type of language,
    // not just the one the project is configured as!
    let copy_from_dir = |source: &Path, dest: &Path, project_lang: LanguageType| -> AppResult {
        let mut files_to_copy: Vec<String> = vec![
            ".prototools".into(),
            CONFIG_PROJECT_FILENAME.into(),
            CONFIG_TEMPLATE_FILENAME.into(),
        ];
        let mut files_to_create: Vec<String> = vec![];

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
                                files_to_create.extend(["src/lib.rs".into(), "src/main.rs".into()]);
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
                    if let Some(typescript_config) = &session.toolchain_config.typescript {
                        files_to_copy.push(typescript_config.project_config_file_name.to_owned());
                        files_to_copy.push(typescript_config.root_config_file_name.to_owned());
                        files_to_copy
                            .push(typescript_config.root_options_config_file_name.to_owned());
                    }
                }
                _ => {}
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

    // Copy moon configuration
    let moon_dir = session.workspace_root.join(CONFIG_DIRNAME);

    debug!(
        scaffold_dir = ?docker_workspace_root,
        moon_dir = ?moon_dir,
        "Copying core moon configuration"
    );

    let moon_configs = glob::walk(moon_dir, ["*.yml", "tasks/**/*.yml"])?
        .into_iter()
        .map(|f| path::to_string(f.strip_prefix(&session.workspace_root).unwrap()))
        .collect::<Result<Vec<String>, _>>()?;

    copy_files(
        &moon_configs,
        &session.workspace_root,
        &docker_workspace_root,
    )?;

    Ok(())
}

#[instrument(skip(session, manifest))]
#[async_recursion]
async fn scaffold_sources_project(
    session: &CliSession,
    docker_sources_root: &Path,
    project_id: &Id,
    manifest: &mut DockerManifest,
) -> AppResult {
    let project = session.get_project_graph().await?.get(project_id)?;

    debug!(
        id = project_id.as_str(),
        "Copying sources from project {}",
        color::id(project_id),
    );

    manifest.focused_projects.insert(project_id.to_owned());

    for file in glob::walk_files(
        &project.root,
        ["**/*", "!node_modules/**", "!target/**/*", "!vendor/**"],
    )? {
        fs::copy_file(
            &file,
            docker_sources_root.join(file.strip_prefix(&session.workspace_root).unwrap()),
        )?;
    }

    for dep_cfg in &project.dependencies {
        // Avoid root-level projects as it will pull in all sources,
        // which is usually not what users want. If they do want it,
        // they can be explicitly on the command line!
        if !dep_cfg.is_root_scope() {
            debug!(
                id = project_id.as_str(),
                dep_id = dep_cfg.id.as_str(),
                "Including dependency project"
            );

            scaffold_sources_project(session, docker_sources_root, &dep_cfg.id, manifest).await?;
        }
    }

    Ok(())
}

#[instrument(skip(session))]
async fn scaffold_sources(
    session: &CliSession,
    docker_root: &Path,
    project_ids: &[Id],
    include: &[String],
) -> AppResult {
    let docker_sources_root = docker_root.join("sources");

    debug!(
        scaffold_dir = ?docker_sources_root,
        projects = ?project_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Scaffolding sources skeleton, including source files from focused projects"
    );

    let mut manifest = DockerManifest {
        focused_projects: FxHashSet::default(),
        unfocused_projects: FxHashSet::default(),
    };

    // Copy all projects
    for project_id in project_ids {
        scaffold_sources_project(session, &docker_sources_root, project_id, &mut manifest).await?;
    }

    // Include non-focused projects in the manifest
    for project_id in session.get_project_graph().await?.ids() {
        if !manifest.focused_projects.contains(project_id) {
            manifest.unfocused_projects.insert(project_id.to_owned());
        }
    }

    // Include via globs
    if !include.is_empty() {
        debug!(
            include = ?include,
            "Including additional sources"
        );

        let files = glob::walk_files(&session.workspace_root, include)?
            .into_iter()
            .map(|f| path::to_string(f.strip_prefix(&session.workspace_root).unwrap()))
            .collect::<Result<Vec<String>, _>>()?;

        copy_files(&files, &session.workspace_root, &docker_sources_root)?;
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
            match line.trim() {
                // Better way?
                ".moon/cache" | ".moon/cache/" | "./.moon/cache" | "./.moon/cache/" => {
                    is_ignored = true;
                    break;
                }
                _ => {}
            };
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
    scaffold_workspace(&session, &docker_root).await?;
    scaffold_sources(&session, &docker_root, &args.ids, &args.include).await?;

    Ok(())
}
