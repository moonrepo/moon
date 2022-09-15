use crate::helpers::load_workspace;
use moon_config::{NodePackageManager, ProjectLanguage};
use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_lang_node::{package::PackageJson, NODE, NPM, PNPM, YARN};
use moon_utils::fs;
use std::path::{Path, PathBuf};

// moon docker scaffold --include *.json

async fn copy_files(list: &[&str], source: &Path, target: &Path) -> Result<(), MoonError> {
    for file in list {
        let source_file = source.join(file);

        if source_file.exists() {
            if source_file.is_dir() {
                fs::copy_dir_all(&source_file, &source_file, &target.join(file)).await?;
            } else {
                fs::copy_file(source_file, target.join(file)).await?;
            }
        }
    }

    Ok(())
}

pub async fn scaffold() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let docker_root = workspace.root.join(CONFIG_DIRNAME).join("docker");

    fs::create_dir_all(&docker_root).await?;

    // Copy the manifest for every project and mimic the folder structure
    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;
        let docker_project_root = docker_root.join(&project.source);

        // Create the project root
        fs::create_dir_all(&docker_project_root).await?;

        // Copy manifest files
        match project.config.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
                let mut files = vec![
                    NPM.manifest_filename,
                    // Copy and arbitrary postinstall scripts
                    "postinstall.js",
                    "postinstall.cjs",
                    "postinstall.mjs",
                ];

                if let Some(typescript_config) = &workspace.config.typescript {
                    files.push(&typescript_config.project_config_file_name);
                }

                copy_files(&files, &project.root, &docker_project_root).await?;
            }
            _ => {}
        }
    }

    // Copy root lockfiles and configurations
    let mut files = vec![];

    if let Some(node_config) = &workspace.config.node {
        let package_manager = match &node_config.package_manager {
            NodePackageManager::Npm => NPM,
            NodePackageManager::Pnpm => PNPM,
            NodePackageManager::Yarn => YARN,
        };

        files.extend_from_slice(package_manager.config_filenames);
        files.extend_from_slice(package_manager.lock_filenames);
        files.push(package_manager.manifest_filename);
    }

    if let Some(typescript_config) = &workspace.config.typescript {
        files.push(&typescript_config.root_config_file_name);
        files.push(&typescript_config.root_options_config_file_name);
    }

    copy_files(&files, &workspace.root, &docker_root).await?;

    Ok(())
}
