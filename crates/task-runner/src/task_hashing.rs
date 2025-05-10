use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_config::{HasherOptimization, UnresolvedVersionSpec};
use moon_hash::{ContentHasher, hash_content};
use moon_pdk_api::{
    HashTaskContentsInput, LocateDependenciesRootInput, LockDependency, ManifestDependency,
    ParseLockInput, ParseLockOutput, ParseManifestInput, ParseManifestOutput,
};
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::TaskHasher;
use moon_toolchain_plugin::ToolchainPlugin;
use rustc_hash::FxHashMap;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;

pub async fn hash_common_task_contents(
    app_context: &AppContext,
    action_context: &ActionContext,
    project: &Project,
    task: &Task,
    hasher: &mut ContentHasher,
) -> miette::Result<()> {
    let mut task_hasher = TaskHasher::new(
        project,
        task,
        &app_context.vcs,
        &app_context.workspace_root,
        &app_context.workspace_config.hasher,
    );

    if task.script.is_none() && action_context.should_inherit_args(&task.target) {
        task_hasher.hash_args(&action_context.passthrough_args);
    }

    task_hasher.hash_deps({
        let mut deps = BTreeMap::default();

        for dep in &task.deps {
            if let Some(entry) = action_context.target_states.get(&dep.target) {
                match entry.get() {
                    TargetState::Passed(hash) => {
                        deps.insert(&dep.target, hash.clone());
                    }
                    TargetState::Passthrough => {
                        deps.insert(&dep.target, "passthrough".into());
                    }
                    _ => {}
                };
            }
        }

        deps
    });

    task_hasher.hash_inputs().await?;

    hasher.hash_content(task_hasher.hash())?;

    Ok(())
}

hash_content!(
    struct TaskToolchainHash<'task> {
        toolchain: &'task str,

        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<&'task UnresolvedVersionSpec>,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        contents: Vec<JsonValue>,

        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        dependencies: FxHashMap<String, String>,
    }
);

pub async fn hash_toolchain_task_contents(
    app_context: &AppContext,
    project: &Project,
    task: &Task,
    hasher: &mut ContentHasher,
) -> miette::Result<()> {
    let registry = &app_context.toolchain_registry;

    // Load all toolchains
    let toolchains = registry
        .load_many(project.get_enabled_toolchains_for_task(task))
        .await?;

    // Loop through toolchains and extract information
    for toolchain in toolchains {
        let mut inject = false;
        let mut content = TaskToolchainHash {
            toolchain: toolchain.id.as_str(),
            contents: vec![],
            dependencies: FxHashMap::default(),
            version: None,
        };

        // Has a version override
        if let Some(version) = project
            .config
            .toolchain
            .plugins
            .get(toolchain.id.as_str())
            .and_then(|config| config.get_version())
        {
            content.version = Some(version);
            inject = true;
        }
        // Or an inherited version
        else if let Some(version) = app_context
            .toolchain_config
            .plugins
            .get(toolchain.id.as_str())
            .and_then(|config| config.version.as_ref())
        {
            content.version = Some(version);
            inject = true;
        }

        // Hash dependencies from manifest
        if apply_toolchain_dependencies(app_context, &toolchain, project, &mut content).await? {
            inject = true;
        }

        // Hash dynamic content
        let output = toolchain
            .hash_task_contents(HashTaskContentsInput {
                context: registry.create_context(),
                project: project.to_fragment(),
                task: task.to_fragment(),
                toolchain_config: registry.create_merged_config(
                    &toolchain.id,
                    &app_context.toolchain_config,
                    &project.config,
                ),
            })
            .await?;

        if !output.contents.is_empty() {
            content.contents = output.contents;
            inject = true;
        }

        // Only hash if we extracted information
        if inject {
            hasher.hash_content(content)?;
        }
    }

    Ok(())
}

async fn apply_toolchain_dependencies(
    app_context: &AppContext,
    toolchain: &ToolchainPlugin,
    project: &Project,
    hash_content: &mut TaskToolchainHash<'_>,
) -> miette::Result<bool> {
    let mut inject = false;
    let mut locked = ParseLockOutput::default();
    let mut workspace_manifest = ParseManifestOutput::default();
    let mut project_manifest = ParseManifestOutput::default();

    // Load the project manifest
    if let Some(manifest_file_name) = &toolchain.metadata.manifest_file_name {
        let manifest_path = project.root.join(manifest_file_name);

        // If the manifest doesn't exist, we can abort early as
        // theres no dependencies to extract!
        if !manifest_path.exists() {
            return Ok(false);
        }

        project_manifest = toolchain
            .parse_manifest(ParseManifestInput {
                context: app_context.toolchain_registry.create_context(),
                path: toolchain.to_virtual_path(manifest_path),
            })
            .await?;
    }

    // Try and locate a dependency root
    let output = toolchain
        .locate_dependencies_root(LocateDependenciesRootInput {
            context: app_context.toolchain_registry.create_context(),
            starting_dir: toolchain.to_virtual_path(&project.root),
        })
        .await?;

    // Found a dependency root
    if let Some(deps_root) = output.root.and_then(|root| root.real_path()) {
        // Parse and extract locked dependencies
        if let Some(lock_file_name) = &toolchain.metadata.lock_file_name {
            let lock_path = deps_root.join(lock_file_name);

            if lock_path.exists()
                && app_context.workspace_config.hasher.optimization == HasherOptimization::Accuracy
            {
                locked = toolchain
                    .parse_lock(ParseLockInput {
                        context: app_context.toolchain_registry.create_context(),
                        path: toolchain.to_virtual_path(lock_path),
                    })
                    .await?;
            }
        }

        // Parse and extract workspace manifest
        if let Some(manifest_file_name) = &toolchain.metadata.manifest_file_name {
            let manifest_path = deps_root.join(manifest_file_name);

            if manifest_path.exists() {
                workspace_manifest = toolchain
                    .parse_manifest(ParseManifestInput {
                        context: app_context.toolchain_registry.create_context(),
                        path: toolchain.to_virtual_path(manifest_path),
                    })
                    .await?;
            }
        }
    }

    // Now extract and hash the dependencies
    if apply_toolchain_dependencies_by_scope(
        project_manifest.peer_dependencies,
        &workspace_manifest.peer_dependencies,
        &locked.dependencies,
        hash_content,
    ) {
        inject = true;
    }

    if apply_toolchain_dependencies_by_scope(
        project_manifest.build_dependencies,
        &workspace_manifest.build_dependencies,
        &locked.dependencies,
        hash_content,
    ) {
        inject = true;
    }

    if apply_toolchain_dependencies_by_scope(
        project_manifest.dev_dependencies,
        &workspace_manifest.dev_dependencies,
        &locked.dependencies,
        hash_content,
    ) {
        inject = true;
    }

    if apply_toolchain_dependencies_by_scope(
        project_manifest.dependencies,
        &workspace_manifest.dependencies,
        &locked.dependencies,
        hash_content,
    ) {
        inject = true;
    }

    Ok(inject)
}

fn apply_toolchain_dependencies_by_scope(
    project_deps: FxHashMap<String, ManifestDependency>,
    workspace_deps: &FxHashMap<String, ManifestDependency>,
    locked_deps: &FxHashMap<String, Vec<LockDependency>>,
    hash_content: &mut TaskToolchainHash<'_>,
) -> bool {
    let mut inject = false;

    for (name, dep) in project_deps {
        let req = if dep.inherited {
            workspace_deps
                .get(&name)
                .and_then(|ws_dep| ws_dep.version.clone())
        } else {
            dep.version
        };

        // If no version requirement, just skip
        let Some(req) = req else {
            continue;
        };

        // Try and find a resolved version from the lock file
        if let Some(lock_deps) = locked_deps.get(&name) {
            if let Some(lock_dep) =
                // By exact version first
                lock_deps
                    .iter()
                    .find(|ld| ld.version.as_ref().is_some_and(|v| &req == v))
                    .or_else(|| {
                        // Then by matching requirement second
                        lock_deps
                            .iter()
                            .find(|ld| ld.req.as_ref().is_some_and(|r| &req == r))
                    })
            {
                // Found, so record a value
                if let Some(hash) = lock_dep
                    .hash
                    .clone()
                    .or_else(|| lock_dep.version.as_ref().map(|v| v.to_string()))
                    .or_else(|| lock_dep.meta.clone())
                {
                    hash_content.dependencies.insert(name, hash);
                    inject = true;

                    continue;
                };
            }
        }

        // None found, so just record the requirement
        hash_content.dependencies.insert(name, req.to_string());
        inject = true;
    }

    inject
}
