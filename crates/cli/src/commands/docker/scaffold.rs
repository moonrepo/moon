use super::MANIFEST_NAME;
use clap::Args;
use moon::{generate_project_graph, load_workspace};
use moon_common::consts::CONFIG_DIRNAME;
use moon_common::Id;
use moon_config::{ConfigEnum, LanguageType};
use moon_platform_detector::detect_language_files;
use moon_project_graph::ProjectGraph;
use moon_rust_lang::cargo_toml::{CargoTomlCache, CargoTomlExt};
use moon_utils::path;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use starbase::{system, AppResult};
use starbase_utils::{fs, glob, json};
use std::path::Path;

#[derive(Args, Clone, Debug)]
pub struct DockerScaffoldArgs {
    #[arg(required = true, help = "List of project IDs to copy sources for")]
    ids: Vec<Id>,

    #[arg(long, help = "Additional file globs to include in sources")]
    include: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub focused_projects: FxHashSet<Id>,
    pub unfocused_projects: FxHashSet<Id>,
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

fn scaffold_workspace(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    docker_root: &Path,
) -> AppResult {
    let docker_workspace_root = docker_root.join("workspace");

    fs::create_dir_all(&docker_workspace_root)?;

    // Copy manifest and config files for every type of language,
    // not just the one the project is configured as!
    let copy_from_dir = |source: &Path, dest: &Path| -> AppResult {
        let mut files: Vec<String> = vec![".prototools".to_owned()];

        for lang in LanguageType::variants() {
            files.extend(detect_language_files(&lang));

            // These are special cases
            match lang {
                LanguageType::Rust => {
                    if let Some(cargo_toml) = CargoTomlCache::read(source)? {
                        let manifests = cargo_toml.get_member_manifest_paths(source)?;

                        for manifest in manifests {
                            files.push(path::to_string(manifest.strip_prefix(source).unwrap())?);
                        }
                    }
                }
                LanguageType::TypeScript => {
                    if let Some(typescript_config) = &workspace.toolchain_config.typescript {
                        files.push(typescript_config.project_config_file_name.to_owned());
                        files.push(typescript_config.root_config_file_name.to_owned());
                        files.push(typescript_config.root_options_config_file_name.to_owned());
                    }
                }
                _ => {}
            }
        }

        copy_files(&files, source, dest)
    };

    // Copy each project and mimic the folder structure
    for source in project_graph.sources().values() {
        if source.as_str() == "." {
            continue;
        }

        let docker_project_root = docker_workspace_root.join(source.as_str());

        fs::create_dir_all(&docker_project_root)?;

        copy_from_dir(&source.to_path(&workspace.root), &docker_project_root)?;
    }

    // Copy root lockfiles and configurations
    copy_from_dir(&workspace.root, &docker_workspace_root)?;

    // Copy moon configuration
    let moon_configs = glob::walk(
        workspace.root.join(CONFIG_DIRNAME),
        ["*.yml", "tasks/*.yml"],
    )?;
    let moon_configs = moon_configs
        .iter()
        .map(|f| path::to_string(f.strip_prefix(&workspace.root).unwrap()))
        .collect::<Result<Vec<String>, _>>()?;

    copy_files(&moon_configs, &workspace.root, &docker_workspace_root)?;

    Ok(())
}

fn scaffold_sources_project(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    docker_sources_root: &Path,
    project_id: &Id,
    manifest: &mut DockerManifest,
) -> AppResult {
    let project = project_graph.get(project_id)?;

    manifest.focused_projects.insert(project_id.to_owned());

    copy_files(&[&project.source], &workspace.root, docker_sources_root)?;

    for dep_id in project.get_dependency_ids() {
        scaffold_sources_project(
            workspace,
            project_graph,
            docker_sources_root,
            dep_id,
            manifest,
        )?;
    }

    Ok(())
}

fn scaffold_sources(
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    docker_root: &Path,
    project_ids: &[Id],
    include: &[String],
) -> AppResult {
    let docker_sources_root = docker_root.join("sources");
    let mut manifest = DockerManifest {
        focused_projects: FxHashSet::default(),
        unfocused_projects: FxHashSet::default(),
    };

    // Copy all projects
    for project_id in project_ids {
        scaffold_sources_project(
            workspace,
            project_graph,
            &docker_sources_root,
            project_id,
            &mut manifest,
        )?;
    }

    // Include non-focused projects in the manifest
    for project_id in project_graph.ids() {
        if !manifest.focused_projects.contains(project_id) {
            manifest.unfocused_projects.insert(project_id.to_owned());
        }
    }

    // Include via globs
    if !include.is_empty() {
        let files = glob::walk_files(&workspace.root, include)?;
        let files = files
            .iter()
            .map(|f| path::to_string(f.strip_prefix(&workspace.root).unwrap()))
            .collect::<Result<Vec<String>, _>>()?;

        copy_files(&files, &workspace.root, &docker_sources_root)?;
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

#[system]
pub async fn scaffold(args: ArgsRef<DockerScaffoldArgs>) {
    let mut workspace = load_workspace().await?;
    let docker_root = workspace.root.join(CONFIG_DIRNAME).join("docker");

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root)?;
    fs::create_dir_all(&docker_root)?;

    // Create the workspace skeleton
    let project_graph = generate_project_graph(&mut workspace).await?;

    scaffold_workspace(&workspace, &project_graph, &docker_root)?;

    scaffold_sources(
        &workspace,
        &project_graph,
        &docker_root,
        &args.ids,
        &args.include,
    )?;
}
