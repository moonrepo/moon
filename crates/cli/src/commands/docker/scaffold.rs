use crate::helpers::load_workspace;
use moon_config::{NodePackageManager, ProjectLanguage};
use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_lang_node::{NODE, NPM, PNPM, YARN};
use moon_utils::fs;
use std::path::Path;
use strum::IntoEnumIterator;

// moon docker scaffold --include *.json --copy-dependencies

async fn copy_files<T: AsRef<str>>(
    list: &[T],
    source: &Path,
    target: &Path,
) -> Result<(), MoonError> {
    for file in list {
        let file = file.as_ref();
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

    // Delete the docker skeleton to remove any stale files
    fs::remove_dir_all(&docker_root).await?;
    fs::create_dir_all(&docker_root).await?;

    // Copy each project and mimic the folder structure
    for project_source in workspace.projects.projects_map.values() {
        let docker_project_root = docker_root.join(&project_source);

        // Create the project root
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
                    files.extend_from_slice(package_manager.config_filenames);
                    files.extend_from_slice(package_manager.lock_filenames);
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

    copy_files(&files, &workspace.root, &docker_root).await?;

    Ok(())
}
