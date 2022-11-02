use crate::helpers::load_workspace;
use async_recursion::async_recursion;
use futures::future::try_join_all;
use moon_config::{NodePackageManager, ProjectID, ProjectLanguage};
use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_node_lang::{NODE, NPM, PNPM, YARN};
use moon_project::ProjectError;
use moon_utils::{fs, glob, path};
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::path::Path;
use strum::IntoEnumIterator;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub focused_projects: FxHashSet<ProjectID>,
    pub unfocused_projects: FxHashSet<ProjectID>,
}

async fn copy_files<T: AsRef<str>>(
    list: &[T],
    source: &Path,
    target: &Path,
) -> Result<(), MoonError> {
    let mut futures = vec![];

    for file in list {
        let file = file.as_ref();
        let source_file = source.join(file);

        if source_file.exists() {
            if source_file.is_dir() {
                fs::copy_dir_all(&source_file, &source_file, &target.join(file)).await?;
            } else {
                futures.push(fs::copy_file(source_file, target.join(file)));
            }
        }
    }

    try_join_all(futures).await?;

    Ok(())
}

async fn scaffold_workspace(workspace: &Workspace, docker_root: &Path) -> Result<(), ProjectError> {
    let docker_workspace_root = docker_root.join("workspace");

    fs::create_dir_all(&docker_workspace_root).await?;

    // Copy each project and mimic the folder structure
    for project_source in workspace.projects.projects_map.values() {
        let docker_project_root = docker_workspace_root.join(&project_source);

        fs::create_dir_all(&docker_project_root).await?;

        // Copy manifest and config files
        let mut files: Vec<String> = vec![];

        for lang in ProjectLanguage::iter() {
            match lang {
                ProjectLanguage::JavaScript => {
                    if workspace.config.node.is_some() {
                        files.push(NPM.manifest_filename.to_owned());

                        for ext in NODE.file_exts {
                            files.push(format!("postinstall.{ext}"));
                        }
                    }
                }
                ProjectLanguage::TypeScript => {
                    if let Some(typescript_config) = &workspace.config.typescript {
                        files.push(typescript_config.project_config_file_name.to_owned());
                    }
                }
                _ => {}
            }
        }

        copy_files(
            &files,
            &workspace.root.join(project_source),
            &docker_project_root,
        )
        .await?;
    }

    // Copy root lockfiles and configurations
    let mut files = vec![];

    for lang in ProjectLanguage::iter() {
        match lang {
            ProjectLanguage::JavaScript => {
                if let Some(node_config) = &workspace.config.node {
                    let package_manager = match &node_config.package_manager {
                        NodePackageManager::Npm => NPM,
                        NodePackageManager::Pnpm => PNPM,
                        NodePackageManager::Yarn => YARN,
                    };

                    files.push(package_manager.manifest_filename);
                    files.push(package_manager.lock_filename);
                    files.extend_from_slice(package_manager.config_filenames);
                }
            }
            ProjectLanguage::TypeScript => {
                if let Some(typescript_config) = &workspace.config.typescript {
                    files.push(&typescript_config.root_config_file_name);
                    files.push(&typescript_config.root_options_config_file_name);
                }
            }
            _ => {}
        }
    }

    copy_files(&files, &workspace.root, &docker_workspace_root).await?;

    // Copy moon configuration
    let moon_configs = glob::walk(&workspace.root.join(CONFIG_DIRNAME), &["*.yml"])?;
    let moon_configs = moon_configs
        .iter()
        .map(|f| path::to_string(f.strip_prefix(&workspace.root).unwrap()))
        .collect::<Result<Vec<String>, MoonError>>()?;

    copy_files(&moon_configs, &workspace.root, &docker_workspace_root).await?;

    Ok(())
}

#[async_recursion]
async fn scaffold_sources_project(
    workspace: &Workspace,
    docker_sources_root: &Path,
    project_id: &str,
    manifest: &mut DockerManifest,
) -> Result<(), ProjectError> {
    let project = workspace.projects.load(project_id)?;

    manifest.focused_projects.insert(project_id.to_owned());

    copy_files(&[&project.source], &workspace.root, docker_sources_root).await?;

    for dep_id in project.get_dependency_ids() {
        scaffold_sources_project(workspace, docker_sources_root, &dep_id, manifest).await?;
    }

    Ok(())
}

async fn scaffold_sources(
    workspace: &Workspace,
    docker_root: &Path,
    project_ids: &[String],
    include: &[String],
) -> Result<(), ProjectError> {
    let docker_sources_root = docker_root.join("sources");
    let mut manifest = DockerManifest {
        focused_projects: FxHashSet::default(),
        unfocused_projects: FxHashSet::default(),
    };

    // Copy all projects
    for project_id in project_ids {
        scaffold_sources_project(workspace, &docker_sources_root, project_id, &mut manifest)
            .await?;
    }

    // Include non-focused projects in the manifest
    for project_id in workspace.projects.projects_map.keys() {
        if !manifest.focused_projects.contains(project_id) {
            manifest.unfocused_projects.insert(project_id.to_owned());
        }
    }

    // Include via globs
    if !include.is_empty() {
        let files = glob::walk(&workspace.root, include)?;
        let files = files
            .iter()
            .map(|f| path::to_string(f.strip_prefix(&workspace.root).unwrap()))
            .collect::<Result<Vec<String>, MoonError>>()?;

        copy_files(&files, &workspace.root, &docker_sources_root).await?;
    }

    fs::write_json(
        docker_sources_root.join("dockerManifest.json"),
        &manifest,
        true,
    )
    .await?;

    Ok(())
}

pub async fn scaffold(
    project_ids: &[String],
    include: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let docker_root = workspace.root.join(CONFIG_DIRNAME).join("docker");

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root).await?;
    fs::create_dir_all(&docker_root).await?;

    // Create the workspace skeleton
    scaffold_workspace(&workspace, &docker_root).await?;
    scaffold_sources(&workspace, &docker_root, project_ids, include).await?;

    Ok(())
}
