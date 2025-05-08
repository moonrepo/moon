use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_common::Id;
use moon_config::UnresolvedVersionSpec;
use moon_hash::{ContentHasher, hash_content};
use moon_pdk_api::{HashTaskContentsInput, LocateDependenciesRootInput};
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::TaskHasher;
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

pub async fn hash_toolchain_task_contents(
    app_context: &AppContext,
    project: &Project,
    task: &Task,
    hasher: &mut ContentHasher,
) -> miette::Result<()> {
    for content in app_context
        .toolchain_registry
        .hash_task_contents_many(
            project.get_enabled_toolchains_for_task(task),
            |registry, toolchain| HashTaskContentsInput {
                context: registry.create_context(),
                project: project.to_fragment(),
                task: task.to_fragment(),
                toolchain_config: registry.create_merged_config(
                    &toolchain.id,
                    &app_context.toolchain_config,
                    &project.config,
                ),
            },
        )
        .await?
    {
        hasher.hash_content(content)?;
    }

    Ok(())
}

hash_content!(
    struct TaskToolchainHash<'task> {
        toolchain: &'task str,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        contents: Vec<JsonValue>,

        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<&'task UnresolvedVersionSpec>,
    }
);

pub async fn hash_toolchain_resolved_dependencies(
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

        // Has a manifest
        if let Some(manifest_file_name) = &toolchain.metadata.manifest_file_name {
            let manifest_path = project.root.join(manifest_file_name);

            // Extract dependencies
            if manifest_path.exists() {
                let lock_path = toolchain.locate_lock_file(&project.root);
            }
        }

        // Provides dynamic hash content
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

    for locate_result in app_context
        .toolchain_registry
        .locate_dependencies_root_many(
            project.get_enabled_toolchains_for_task(task),
            |registry, toolchain| LocateDependenciesRootInput {
                context: registry.create_context(),
                starting_dir: toolchain.to_virtual_path(&project.root),
            },
        )
        .await?
    {}

    Ok(())
}
