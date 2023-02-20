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
use std::{collections::BTreeMap, path::Path};

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

    let tsconfig_options_path = workspace_root.join(&tsconfig_options_name);

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
    TsConfigJson::sync_with_name(workspace_root, Some(tsconfig_root_name), |tsconfig_json| {
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
    route_to_cache: bool,
    sync_paths: bool,
) -> Result<bool, MoonError> {
    TsConfigJson::sync_with_name(
        &project.root,
        Some(&tsconfig_project_name),
        |tsconfig_json| {
            let mut mutated_tsconfig = false;

            // Project references
            if !tsconfig_project_refs.is_empty() {
                for ref_path in tsconfig_project_refs {
                    if tsconfig_json.add_project_ref(&ref_path, &tsconfig_project_name) {
                        mutated_tsconfig = true;
                    }
                }
            }

            // Out dir
            if route_to_cache {
                let cache_route = get_cache_dir().join("types").join(&project.source);
                let out_dir = path::to_virtual_string(
                    path::relative_from(cache_route, &project.root).unwrap(),
                )?;
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
            if sync_paths
                && !tsconfig_paths.is_empty()
                && tsconfig_json
                    .update_compiler_options(|options| options.update_paths(tsconfig_paths))
            {
                mutated_tsconfig = true;
            }

            Ok(mutated_tsconfig)
        },
    )
}

pub fn sync_project(
    project: &Project,
    typescript_config: &TypeScriptConfig,
    workspace_root: &Path,
    tsconfig_paths: CompilerOptionsPaths,
    tsconfig_project_refs: FxHashSet<String>,
) -> Result<bool, ProjectError> {
    let tsconfig_project_name = typescript_config
        .project_config_file_name
        .clone()
        .unwrap_or_else(|| "tsconfig.json".into());

    let tsconfig_options_name = typescript_config
        .root_options_config_file_name
        .clone()
        .unwrap_or_else(|| "tsconfig.options.json".into());

    let tsconfig_root_name = typescript_config
        .root_config_file_name
        .clone()
        .unwrap_or_else(|| "tsconfig.json".into());

    let is_project_typescript_enabled = project.config.toolchain.typescript;
    let mut mutated_tsconfig = false;

    // Auto-create a `tsconfig.json` if configured and applicable
    if is_project_typescript_enabled
        && typescript_config.sync_project_references
        && typescript_config.create_missing_config
        && !project.root.join(&tsconfig_project_name).exists()
    {
        if create_missing_tsconfig(
            project,
            &tsconfig_project_name,
            &tsconfig_options_name,
            workspace_root,
        )? {
            mutated_tsconfig = true;
        }
    }

    // Sync compiler options to the project's `tsconfig.json`
    if is_project_typescript_enabled {
        if sync_project_tsconfig_compiler_options(
            project,
            &tsconfig_project_name,
            tsconfig_paths,
            tsconfig_project_refs,
            typescript_config.route_out_dir_to_cache,
            typescript_config.sync_project_references_to_paths,
        )? {
            mutated_tsconfig = true;
        }
    }

    // Sync project references to the root `tsconfig.json`
    if is_project_typescript_enabled && typescript_config.sync_project_references {
        if sync_root_tsconfig_references(
            project,
            &tsconfig_project_name,
            &tsconfig_root_name,
            workspace_root,
        )? {
            mutated_tsconfig = true;
        }
    }

    Ok(mutated_tsconfig)
}

pub fn sync_dependency_project(
    project: &Project,
    dep_project: &Project,
) -> Result<(CompilerOptionsPaths, FxHashSet<String>), ProjectError> {
    let mut tsconfig_project_refs: FxHashSet<String> = FxHashSet::default();
    let mut tsconfig_paths: CompilerOptionsPaths = BTreeMap::new();
    let is_project_typescript_enabled = project.config.toolchain.typescript;
    let is_dep_typescript_enabled = dep_project.config.toolchain.typescript;

    Ok((tsconfig_paths, tsconfig_project_refs))
}
