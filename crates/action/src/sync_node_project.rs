use crate::action::{Action, ActionStatus};
use crate::context::ActionContext;
use crate::errors::ActionError;
use moon_config::TypeScriptConfig;
use moon_lang_node::{package::PackageJson, tsconfig::TsConfigJson};
use moon_logger::{color, debug};
use moon_project::Project;
use moon_utils::{fs, is_ci, path, string_vec};
use moon_workspace::Workspace;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:sync-node-project";

// Automatically create missing config files when we are syncing project references.
#[track_caller]
async fn create_missing_tsconfig(
    project: &Project,
    typescript_config: &TypeScriptConfig,
    workspace_root: &Path,
) -> Result<bool, ActionError> {
    let tsconfig_path = project
        .root
        .join(&typescript_config.project_config_file_name);

    if tsconfig_path.exists() {
        return Ok(false);
    }

    let tsconfig_options_path =
        workspace_root.join(&typescript_config.root_options_config_file_name);

    let json = TsConfigJson {
        extends: Some(path::to_virtual_string(
            path::relative_from(&tsconfig_options_path, &project.root).unwrap(),
        )?),
        include: Some(string_vec!["**/*"]),
        references: Some(vec![]),
        path: tsconfig_path.clone(),
        ..TsConfigJson::default()
    };

    fs::write_json(&tsconfig_path, &json, true).await?;

    Ok(true)
}

// Sync projects references to the root `tsconfig.json`.
fn sync_root_tsconfig(
    tsconfig: &mut TsConfigJson,
    typescript_config: &TypeScriptConfig,
    project: &Project,
) -> bool {
    if project
        .root
        .join(&typescript_config.project_config_file_name)
        .exists()
        && tsconfig.add_project_ref(&project.source, &typescript_config.project_config_file_name)
    {
        debug!(
            target: LOG_TARGET,
            "Syncing {} as a project reference to the root {}",
            color::id(&project.id),
            color::file(&typescript_config.root_config_file_name)
        );

        return true;
    }

    false
}

pub async fn sync_node_project(
    _action: &mut Action,
    _context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    project_id: &str,
) -> Result<ActionStatus, ActionError> {
    let mut mutated_files = false;
    let workspace = workspace.read().await;
    let node_config = &workspace.config.node;
    let typescript_config = &workspace.config.typescript;
    let project = workspace.projects.load(project_id)?;

    // Auto-create a `tsconfig.json` if configured and applicable
    if typescript_config.create_missing_config
        && typescript_config.sync_project_references
        && !project
            .root
            .join(&typescript_config.project_config_file_name)
            .exists()
    {
        create_missing_tsconfig(&project, typescript_config, &workspace.root).await?;
    }

    // Sync each dependency to `tsconfig.json` and `package.json`
    let dep_version_range = workspace
        .toolchain
        .get_node()
        .get_package_manager()
        .get_workspace_dependency_range();

    for dep_id in project.get_dependencies() {
        let dep_project = workspace.projects.load(&dep_id)?;

        // Update `dependencies` within this project's `package.json`.
        // Only add if the dependent project has a `package.json`,
        // and this `package.json` has not already declared the dep.
        if node_config.sync_project_workspace_dependencies {
            if let Some(dep_package_json) = PackageJson::read(&dep_project.root).await? {
                PackageJson::sync(&project.root, |package_json| {
                    if package_json.add_dependency(
                        &dep_package_json.name.unwrap_or_default(),
                        &dep_version_range,
                        true,
                    ) {
                        mutated_files = true;

                        debug!(
                            target: LOG_TARGET,
                            "Syncing {} as a dependency to {}'s {}",
                            color::id(&dep_id),
                            color::id(project_id),
                            color::file("package.json")
                        );
                    }

                    Ok(())
                })
                .await?;
            }
        }

        // Update `references` within this project's `tsconfig.json`.
        // Only add if the dependent project has a `tsconfig.json`,
        // and this `tsconfig.json` has not already declared the dep.
        if typescript_config.sync_project_references {
            let tsconfig_branch_name = &typescript_config.project_config_file_name;

            if dep_project.root.join(tsconfig_branch_name).exists() {
                TsConfigJson::sync_with_name(
                    &project.root,
                    &tsconfig_branch_name,
                    |tsconfig_json| {
                        let dep_ref_path = path::to_string(
                            path::relative_from(&dep_project.root, &project.root)
                                .unwrap_or_default(),
                        )?;

                        if tsconfig_json.add_project_ref(&dep_ref_path, tsconfig_branch_name) {
                            mutated_files = true;

                            debug!(
                                target: LOG_TARGET,
                                "Syncing {} as a project reference to {}'s {}",
                                color::id(&dep_id),
                                color::id(project_id),
                                color::file(tsconfig_branch_name)
                            );
                        }

                        Ok(())
                    },
                )
                .await?;
            }
        }
    }

    // Sync to the root `tsconfig.json` (only if the project has a tsconfig)
    if typescript_config.sync_project_references {
        TsConfigJson::sync_with_name(
            &workspace.root,
            &typescript_config.root_config_file_name,
            |tsconfig_json| {
                if sync_root_tsconfig(tsconfig_json, typescript_config, &project) {
                    mutated_files = true;
                }

                Ok(())
            },
        )
        .await?;
    }

    if mutated_files {
        // If files have been modified in CI, we should update the status to warning,
        // as these modifications should be committed to the repo.
        if is_ci() {
            return Ok(ActionStatus::Invalid);
        } else {
            return Ok(ActionStatus::Passed);
        }
    }

    Ok(ActionStatus::Skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_config::GlobalProjectConfig;
    use moon_utils::test::create_fixtures_skeleton_sandbox;

    #[tokio::test]
    async fn creates_tsconfig() {
        let fixture = create_fixtures_skeleton_sandbox("cases");

        let project = Project::new(
            "deps-a",
            "deps-a",
            fixture.path(),
            &GlobalProjectConfig::default(),
            &[],
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(&project, &TypeScriptConfig::default(), fixture.path())
            .await
            .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read(tsconfig_path).await.unwrap().unwrap();

        assert_eq!(
            tsconfig.extends,
            Some("../tsconfig.options.json".to_owned())
        );
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[tokio::test]
    async fn creates_tsconfig_with_custom_settings() {
        let fixture = create_fixtures_skeleton_sandbox("cases");

        let project = Project::new(
            "deps-a",
            "deps-a",
            fixture.path(),
            &GlobalProjectConfig::default(),
            &[],
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.ref.json");

        assert!(!tsconfig_path.exists());

        create_missing_tsconfig(
            &project,
            &TypeScriptConfig {
                project_config_file_name: "tsconfig.ref.json".to_string(),
                root_options_config_file_name: "tsconfig.base.json".to_string(),
                ..TypeScriptConfig::default()
            },
            fixture.path(),
        )
        .await
        .unwrap();

        assert!(tsconfig_path.exists());

        let tsconfig = TsConfigJson::read_with_name(&project.root, "tsconfig.ref.json")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(tsconfig.extends, Some("../tsconfig.base.json".to_owned()));
        assert_eq!(tsconfig.include, Some(string_vec!["**/*"]));
    }

    #[tokio::test]
    async fn doesnt_create_if_a_config_exists() {
        let fixture = create_fixtures_skeleton_sandbox("cases");

        let project = Project::new(
            "deps-b",
            "deps-b",
            fixture.path(),
            &GlobalProjectConfig::default(),
            &[],
        )
        .unwrap();

        let tsconfig_path = project.root.join("tsconfig.json");

        assert!(tsconfig_path.exists());

        let created =
            create_missing_tsconfig(&project, &TypeScriptConfig::default(), fixture.path())
                .await
                .unwrap();

        assert!(!created);
    }
}
