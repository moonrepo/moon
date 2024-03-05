use moon_common::Id;
use moon_config::{DependencyScope, TypeScriptConfig};
use moon_node_lang::PackageJson;
use moon_project::Project;
use moon_typescript_lang::{
    tsconfig::{CompilerOptionsPathsMap, ExtendsField, PathOrGlob},
    TsConfigJson, TsConfigJsonCache,
};
use moon_utils::{
    get_cache_dir,
    path::{self, to_relative_virtual_string},
};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_styles::color;
use starbase_utils::json;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::debug;

pub struct TypeScriptSyncer<'app> {
    project: &'app Project,
    typescript_config: &'app TypeScriptConfig,
    types_root: PathBuf,
}

impl<'app> TypeScriptSyncer<'app> {
    pub fn new(
        project: &'app Project,
        typescript_config: &'app TypeScriptConfig,
        workspace_root: &'app Path,
    ) -> Self {
        Self {
            types_root: path::normalize(workspace_root.join(&typescript_config.root)),
            project,
            typescript_config,
        }
    }

    pub fn should_include_project_reference_sources(&self) -> bool {
        self.project
            .config
            .toolchain
            .typescript
            .as_ref()
            .and_then(|cfg| cfg.include_project_reference_sources)
            .unwrap_or(self.typescript_config.include_project_reference_sources)
    }

    pub fn should_include_shared_types(&self) -> bool {
        self.project
            .config
            .toolchain
            .typescript
            .as_ref()
            .and_then(|cfg| cfg.include_shared_types)
            .unwrap_or(self.typescript_config.include_shared_types)
    }

    pub fn should_route_out_dir_to_cache(&self) -> bool {
        self.project
            .config
            .toolchain
            .typescript
            .as_ref()
            .and_then(|cfg| cfg.route_out_dir_to_cache)
            .unwrap_or(self.typescript_config.route_out_dir_to_cache)
    }

    pub fn should_sync_project_references(&self) -> bool {
        self.project
            .config
            .toolchain
            .typescript
            .as_ref()
            .and_then(|cfg| cfg.sync_project_references)
            .unwrap_or(self.typescript_config.sync_project_references)
    }

    pub fn should_sync_project_references_to_paths(&self) -> bool {
        self.project
            .config
            .toolchain
            .typescript
            .as_ref()
            .and_then(|cfg| cfg.sync_project_references_to_paths)
            .unwrap_or(self.typescript_config.sync_project_references_to_paths)
    }

    // Automatically create a missing `tsconfig.json` when we are syncing project references.
    pub fn create_missing_tsconfig(&self) -> miette::Result<bool> {
        let tsconfig_path = self
            .project
            .root
            .join(&self.typescript_config.project_config_file_name);

        if tsconfig_path.exists() {
            return Ok(false);
        }

        let json = TsConfigJson {
            extends: Some(ExtendsField::Single(path::to_relative_virtual_string(
                self.types_root
                    .join(&self.typescript_config.root_options_config_file_name),
                &self.project.root,
            )?)),
            include: Some(vec![PathOrGlob::Glob("**/*".into())]),
            references: Some(vec![]),
            // path: tsconfig_path.clone(),
            ..TsConfigJson::default()
        };

        json::write_file(&tsconfig_path, &json, true)?;

        Ok(true)
    }

    // Sync project as a reference to the root `tsconfig.json`.
    pub fn sync_as_root_project_reference(&self) -> miette::Result<bool> {
        let tsconfig_root_name = &self.typescript_config.root_config_file_name;
        let tsconfig_project_name = &self.typescript_config.project_config_file_name;

        TsConfigJsonCache::sync_with_name(&self.types_root, tsconfig_root_name, |tsconfig_json| {
            // Don't sync a root project to itself
            if self.project.root == self.types_root && tsconfig_project_name == tsconfig_root_name {
                return Ok(false);
            }

            if self.project.root.join(tsconfig_project_name).exists()
                && tsconfig_json.add_project_ref(&self.project.root, tsconfig_project_name)?
            {
                debug!(
                    "Syncing {} as a project reference to the root {}",
                    color::id(&self.project.id),
                    tsconfig_root_name
                );

                return Ok(true);
            }

            Ok(false)
        })
    }

    // Sync a project's `tsconfig.json`.
    pub fn sync_project_tsconfig(
        &self,
        tsconfig_project_refs: FxHashSet<PathBuf>,
    ) -> miette::Result<bool> {
        TsConfigJsonCache::sync_with_name(
            &self.project.root,
            &self.typescript_config.project_config_file_name,
            |tsconfig_json| {
                let mut mutated_tsconfig = false;
                let should_include_sources = self.should_include_project_reference_sources();
                let should_sync_paths = self.should_sync_project_references_to_paths();

                // Add shared types to include
                if self.should_include_shared_types()
                    && self.types_root.join("types").exists()
                    && tsconfig_json.add_include_path(self.types_root.join("types/**/*"))?
                {
                    mutated_tsconfig = true;
                }

                // Sync dependencies as project references
                if self.should_sync_project_references() && !tsconfig_project_refs.is_empty() {
                    for ref_path in tsconfig_project_refs {
                        if tsconfig_json.add_project_ref(
                            ref_path,
                            &self.typescript_config.project_config_file_name,
                        )? {
                            mutated_tsconfig = true;
                        }
                    }
                }

                // Map all project references (not just synced) to other fields
                if should_include_sources || should_sync_paths {
                    if let Some(local_project_refs) = tsconfig_json.data.references.clone() {
                        let mut tsconfig_compiler_paths = CompilerOptionsPathsMap::default();

                        for project_ref in local_project_refs {
                            let mut abs_ref =
                                path::normalize(self.project.root.join(&project_ref.path));

                            // Remove the tsconfig.json file name if it exists
                            if project_ref.path.ends_with(".json") {
                                abs_ref = abs_ref.parent().unwrap().to_path_buf();
                            }

                            // include
                            if should_include_sources
                                && tsconfig_json.add_include_path(abs_ref.join("**/*"))?
                            {
                                mutated_tsconfig = true;
                            }

                            // paths
                            if should_sync_paths {
                                if let Some(dep_package_json) = PackageJson::read(&abs_ref)? {
                                    if let Some(dep_package_name) = &dep_package_json.name {
                                        for index in [
                                            "src/index.ts",
                                            "src/index.tsx",
                                            "index.ts",
                                            "index.tsx",
                                        ] {
                                            if abs_ref.join(index).exists() {
                                                tsconfig_compiler_paths.insert(
                                                    dep_package_name.clone(),
                                                    vec![to_relative_virtual_string(
                                                        abs_ref.join(index),
                                                        &self.project.root,
                                                    )?],
                                                );

                                                break;
                                            }
                                        }

                                        tsconfig_compiler_paths.insert(
                                            format!("{dep_package_name}/*"),
                                            vec![to_relative_virtual_string(
                                                abs_ref.join(if abs_ref.join("src").exists() {
                                                    "src/*"
                                                } else {
                                                    "*"
                                                }),
                                                &self.project.root,
                                            )?],
                                        );
                                    }
                                }
                            }
                        }

                        // paths
                        if should_sync_paths
                            && tsconfig_json.update_compiler_option_paths(tsconfig_compiler_paths)
                        {
                            mutated_tsconfig = true;
                        }
                    }
                }

                // Route outDir to moon's cache
                if self.should_route_out_dir_to_cache() {
                    let cache_route = get_cache_dir()
                        .join("types")
                        .join(self.project.source.as_str());
                    let out_dir = PathBuf::from(path::to_relative_virtual_string(
                        cache_route,
                        &self.project.root,
                    )?);

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

                Ok(mutated_tsconfig)
            },
        )
    }

    pub fn sync(&self, dependencies: &FxHashMap<Id, Arc<Project>>) -> miette::Result<bool> {
        let mut mutated = false;

        if !self.project.config.toolchain.is_typescript_enabled() {
            return Ok(mutated);
        }

        // Sync each dependency to `tsconfig.json` and `package.json`
        let mut tsconfig_project_refs: FxHashSet<PathBuf> = FxHashSet::default();

        for dep_config in &self.project.dependencies {
            let Some(dep_project) = dependencies.get(&dep_config.id) else {
                continue;
            };

            if dep_project.is_root_level() || matches!(dep_config.scope, DependencyScope::Root) {
                continue;
            }

            // Update `references` within this project's `tsconfig.json`.
            // Only add if the dependent project has a `tsconfig.json`,
            // and this `tsconfig.json` has not already declared the dep.
            if dep_project.config.toolchain.is_typescript_enabled()
                && dep_project
                    .root
                    .join(&self.typescript_config.project_config_file_name)
                    .exists()
            {
                tsconfig_project_refs.insert(dep_project.root.clone());

                debug!(
                    "Syncing {} as a project reference to {}'s {}",
                    color::id(&dep_project.id),
                    color::id(&self.project.id),
                    self.typescript_config.project_config_file_name
                );
            }
        }

        if self.should_sync_project_references() {
            // Auto-create a `tsconfig.json` if configured and applicable
            if self.typescript_config.create_missing_config && self.create_missing_tsconfig()? {
                mutated = true;
            }

            // Sync project reference to the root `tsconfig.json`
            if self.sync_as_root_project_reference()? {
                mutated = true;
            }
        }

        // Sync compiler options to the project's `tsconfig.json`
        if self.sync_project_tsconfig(tsconfig_project_refs)? {
            mutated = true;
        }

        Ok(mutated)
    }
}
