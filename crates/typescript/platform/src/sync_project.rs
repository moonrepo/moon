use moon_config::TypeScriptConfig;
use moon_error::MoonError;
use moon_logger::{color, debug};
use moon_project::{Project, ProjectError};
use moon_typescript_lang::{
    tsconfig::{CompilerOptionsPaths, TsConfigExtends},
    TsConfigJson,
};
use moon_utils::{get_cache_dir, json, path, string_vec};
use rustc_hash::FxHashSet;
use std::path::Path;

const LOG_TARGET: &str = "moon:typescript-platform:sync-project";

// Automatically create a missing `tsconfig.json` when we are syncing project references.
#[track_caller]
pub fn create_missing_tsconfig(
    project: &Project,
    tsconfig_project_name: &str,
    tsconfig_options_name: &str,
    workspace_root: &Path,
) -> Result<bool, MoonError> {
    let tsconfig_path = project.root.join(tsconfig_project_name);

    if tsconfig_path.exists() {
        return Ok(false);
    }

    let tsconfig_options_path = workspace_root.join(tsconfig_options_name);

    let json = TsConfigJson {
        extends: Some(TsConfigExtends::String(path::to_virtual_string(
            path::relative_from(tsconfig_options_path, &project.root).unwrap(),
        )?)),
        include: Some(string_vec!["**/*"]),
        references: Some(vec![]),
        path: tsconfig_path.clone(),
        ..TsConfigJson::default()
    };

    json::write(&tsconfig_path, &json, true)?;

    Ok(true)
}

// Sync projects references to the root `tsconfig.json`.
pub fn sync_root_tsconfig_references(
    project: &Project,
    tsconfig_project_name: &str,
    tsconfig_root_name: &str,
    workspace_root: &Path,
) -> Result<bool, MoonError> {
    TsConfigJson::sync_with_name(workspace_root, tsconfig_root_name, |tsconfig_json| {
        if project.root.join(tsconfig_project_name).exists()
            && tsconfig_json.add_project_ref(&project.source, tsconfig_project_name)
        {
            debug!(
                target: LOG_TARGET,
                "Syncing {} as a project reference to the root {}",
                color::id(&project.id),
                color::file(tsconfig_root_name)
            );

            return Ok(true);
        }

        Ok(false)
    })
}

// Sync compiler options to a project's `tsconfig.json`.
pub fn sync_project_tsconfig_compiler_options(
    project: &Project,
    tsconfig_project_name: &str,
    tsconfig_paths: CompilerOptionsPaths,
    tsconfig_project_refs: FxHashSet<String>,
    setting_route_to_cache: bool,
    setting_sync_project_refs: bool,
    setting_sync_path_aliases: bool,
) -> Result<bool, MoonError> {
    TsConfigJson::sync_with_name(&project.root, tsconfig_project_name, |tsconfig_json| {
        let mut mutated_tsconfig = false;

        // Project references
        if setting_sync_project_refs && !tsconfig_project_refs.is_empty() {
            for ref_path in tsconfig_project_refs {
                if tsconfig_json.add_project_ref(&ref_path, tsconfig_project_name) {
                    mutated_tsconfig = true;
                }
            }
        }

        // Out dir
        if setting_route_to_cache {
            let cache_route = get_cache_dir().join("types").join(&project.source);
            let out_dir =
                path::to_virtual_string(path::relative_from(cache_route, &project.root).unwrap())?;
            let updated_options = tsconfig_json.update_compiler_options(|options| {
                if options.out_dir.is_none() || options.out_dir.as_ref() != Some(&out_dir) {
                    options.out_dir = Some(out_dir);

                    return true;
                }

                false
            });

            if updated_options {
                mutated_tsconfig = true;
            }
        }

        // Paths
        if setting_sync_path_aliases
            && !tsconfig_paths.is_empty()
            && tsconfig_json.update_compiler_options(|options| options.update_paths(tsconfig_paths))
        {
            mutated_tsconfig = true;
        }

        Ok(mutated_tsconfig)
    })
}

pub fn sync_project(
    project: &Project,
    typescript_config: &TypeScriptConfig,
    workspace_root: &Path,
    tsconfig_paths: CompilerOptionsPaths,
    tsconfig_project_refs: FxHashSet<String>,
) -> Result<bool, ProjectError> {
    let is_project_typescript_enabled = project.config.toolchain.is_typescript_enabled();
    let mut mutated_tsconfig = false;

    // Determine settings
    let mut setting_route_to_cache = typescript_config.route_out_dir_to_cache;
    let mut setting_sync_project_refs = typescript_config.sync_project_references;
    let mut setting_sync_path_aliases = typescript_config.sync_project_references_to_paths;

    if let Some(project_typescript_config) = &project.config.toolchain.typescript {
        setting_route_to_cache = project_typescript_config
            .route_out_dir_to_cache
            .unwrap_or(setting_route_to_cache);

        setting_sync_project_refs = project_typescript_config
            .sync_project_references
            .unwrap_or(setting_sync_project_refs);

        setting_sync_path_aliases = project_typescript_config
            .sync_project_references_to_paths
            .unwrap_or(setting_sync_path_aliases);
    }

    // Auto-create a `tsconfig.json` if configured and applicable
    if is_project_typescript_enabled
        && setting_sync_project_refs
        && typescript_config.create_missing_config
        && !project
            .root
            .join(&typescript_config.project_config_file_name)
            .exists()
        && create_missing_tsconfig(
            project,
            &typescript_config.project_config_file_name,
            &typescript_config.root_options_config_file_name,
            workspace_root,
        )?
    {
        mutated_tsconfig = true;
    }

    // Sync compiler options to the project's `tsconfig.json`
    if is_project_typescript_enabled
        && sync_project_tsconfig_compiler_options(
            project,
            &typescript_config.project_config_file_name,
            tsconfig_paths,
            tsconfig_project_refs,
            setting_route_to_cache,
            setting_sync_project_refs,
            setting_sync_path_aliases,
        )?
    {
        mutated_tsconfig = true;
    }

    // Sync project references to the root `tsconfig.json`
    if is_project_typescript_enabled
        && setting_sync_project_refs
        && sync_root_tsconfig_references(
            project,
            &typescript_config.project_config_file_name,
            &typescript_config.root_config_file_name,
            workspace_root,
        )?
    {
        mutated_tsconfig = true;
    }

    Ok(mutated_tsconfig)
}
