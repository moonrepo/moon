use crate::task_hasher::TaskHasher;
use miette::IntoDiagnostic;
use moon_action::ActionNode;
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_config::{
    DependencyScope, HasherOptimization, ProjectConfig, TaskDependencyCacheStrategy,
    UnresolvedVersionSpec,
};
use moon_hash::{ContentHasher, fingerprint};
use moon_pdk_api::{
    HashTaskContentsInput, LocateDependenciesRootInput, LockDependency, ManifestDependency,
    ParseLockInput, ParseLockOutput, ParseManifestInput, ParseManifestOutput,
};
use moon_project::{Project, ProjectFragment};
use moon_task::{Task, TaskFragment};
use moon_toolchain_plugin::ToolchainPlugin;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;

pub async fn hash_common_task_contents(
    app_context: &AppContext,
    action_context: &ActionContext,
    project: &Project,
    task: &Task,
    node: &ActionNode,
    hasher: &mut ContentHasher,
) -> miette::Result<()> {
    let mut task_hasher = TaskHasher::new(
        app_context,
        project,
        task,
        &app_context.workspace_config.hasher,
    );

    if task.script.is_none() && action_context.should_inherit_args(&task.target) {
        task_hasher.hash_args(&action_context.passthrough_args);
    }

    task_hasher.hash_deps({
        let mut deps = BTreeMap::default();

        for dep in &task.deps {
            if action_context.is_dependency_ignored(&task.target, &dep.target) {
                deps.insert(&dep.target, "passthrough".into());
                continue;
            }

            if let Some(entry) = action_context.target_states.get_sync(&dep.target)
                && let Some(value) = dep_hash_input(dep.cache_strategy, entry.get())
            {
                deps.insert(&dep.target, value);
            }
        }

        deps
    });

    task_hasher.hash_inputs().await?;

    if let ActionNode::RunTask(inner) = node {
        task_hasher.hash_args(&inner.args);
        task_hasher.hash_env(&inner.env);
    }

    hasher.hash_content(task_hasher.hash())?;

    Ok(())
}

/// Returns the value a dep contributes to the consumer task's hash, based on
/// the dep's `cache_strategy` and its observed `TargetState`. Returns `None`
/// when the dep should not contribute at all (e.g. failed or skipped runs
/// that can't be relied on for cache correctness).
pub fn dep_hash_input(
    strategy: Option<TaskDependencyCacheStrategy>,
    state: &TargetState,
) -> Option<String> {
    match strategy {
        // Ignored: constant marker — dep changes never invalidate this task.
        Some(TaskDependencyCacheStrategy::Ignored) => Some("ignored".into()),
        // Hash: mix the dep's actual hash so any dep change invalidates. `None`
        // is a defensive backstop — all deps should have a resolved cache
        // strategy by this point.
        Some(TaskDependencyCacheStrategy::Hash) | None => match state {
            TargetState::Passed(hash) | TargetState::SkippedConditional(hash) => Some(hash.clone()),
            TargetState::Passthrough => Some("passthrough".into()),
            TargetState::Failed | TargetState::Skipped => None,
        },
        // Outputs: constant marker — invalidation comes from the dep's outputs,
        // which `inject_deps_outputs` adds to this task's inputs.
        Some(TaskDependencyCacheStrategy::Outputs) => match state {
            TargetState::Passed(_) | TargetState::SkippedConditional(_) => Some("outputs".into()),
            TargetState::Passthrough => Some("outputs-passthrough".into()),
            TargetState::Failed | TargetState::Skipped => None,
        },
    }
}

fingerprint!(
    struct TaskToolchainFingerprint {
        toolchain: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<UnresolvedVersionSpec>,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        contents: Vec<JsonValue>,

        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        dependencies: BTreeMap<DependencyScope, BTreeMap<String, String>>,
    }
);

pub async fn hash_toolchain_task_contents(
    app_context: &Arc<AppContext>,
    project: &Project,
    task: &Task,
    hasher: &mut ContentHasher,
) -> miette::Result<()> {
    // Load all toolchains
    let toolchains = app_context
        .toolchain_registry
        .load_many(project.get_enabled_toolchains_for_task(task))
        .await?;

    // Loop through toolchains and hash information
    let project_config = Arc::new(project.config.clone());
    let mut contents = vec![];
    let mut set = JoinSet::new();

    for toolchain in toolchains {
        let app_context = Arc::clone(app_context);
        let project_frag = project.to_fragment();
        let project_config = Arc::clone(&project_config);
        let task_frag = task.to_fragment();

        set.spawn(async {
            apply_toolchain(
                app_context,
                toolchain,
                project_frag,
                project_config,
                task_frag,
            )
            .await
        });
    }

    while let Some(result) = set.join_next().await {
        if let Some(content) = result.into_diagnostic()?? {
            contents.push(content);
        }
    }

    // Sort the contents so the hash is deterministic
    contents.sort_by(|a, d| a.toolchain.cmp(&d.toolchain));

    for content in contents {
        hasher.hash_content(content)?;
    }

    Ok(())
}

async fn apply_toolchain(
    app_context: Arc<AppContext>,
    toolchain: Arc<ToolchainPlugin>,
    project: ProjectFragment,
    project_config: Arc<ProjectConfig>,
    task: TaskFragment,
) -> miette::Result<Option<TaskToolchainFingerprint>> {
    let mut inject = false;
    let mut fingerprint = TaskToolchainFingerprint {
        toolchain: toolchain.id.to_string(),
        contents: vec![],
        dependencies: BTreeMap::default(),
        version: None,
    };

    // Has a version override
    if let Some(version) = project_config
        .toolchains
        .get_plugin_config(toolchain.id.as_str())
        .and_then(|config| config.get_version())
    {
        fingerprint.version = Some(version.to_owned());
        inject = true;
    }
    // Or an inherited version
    else if let Some(version) = app_context
        .toolchains_config
        .get_plugin_config(toolchain.id.as_str())
        .and_then(|config| config.version.as_ref())
    {
        fingerprint.version = Some(version.to_owned());
        inject = true;
    }

    // Hash dependencies from manifest
    if apply_toolchain_dependencies(
        &app_context,
        &toolchain,
        app_context.workspace_root.join(&project.source),
        &project_config,
        &mut fingerprint,
    )
    .await?
    {
        inject = true;
    }

    // Hash dynamic content
    if toolchain.has_func("hash_task_contents").await {
        let output = toolchain
            .hash_task_contents(HashTaskContentsInput {
                context: app_context.toolchain_registry.create_context(),
                project,
                task,
                toolchain_config: app_context
                    .toolchain_registry
                    .create_merged_config(&toolchain.id, &project_config),
            })
            .await?;

        if !output.contents.is_empty() {
            fingerprint.contents = output.contents;
            inject = true;
        }
    }

    Ok(if inject { Some(fingerprint) } else { None })
}

async fn apply_toolchain_dependencies(
    app_context: &AppContext,
    toolchain: &ToolchainPlugin,
    project_root: PathBuf,
    project_config: &ProjectConfig,
    fingerprint: &mut TaskToolchainFingerprint,
) -> miette::Result<bool> {
    let mut lock_files = vec![];
    let mut workspace_manifests = vec![];
    let mut project_manifests = vec![];
    let has_parse_manifest = toolchain.has_func("parse_manifest").await;
    let has_parse_lock = toolchain.has_func("parse_lock").await;

    // Try and locate a dependency root
    let output = if toolchain.has_func("locate_dependencies_root").await {
        toolchain
            .locate_dependencies_root(LocateDependenciesRootInput {
                context: app_context.toolchain_registry.create_context(),
                starting_dir: toolchain.to_virtual_path(&project_root),
                toolchain_config: app_context
                    .toolchain_registry
                    .create_merged_config(&toolchain.id, project_config),
            })
            .await?
    } else {
        Default::default()
    };

    // Found a dependency root
    if let Some(deps_root) = output.root {
        // Parse and extract locked dependencies
        for lock_file_name in &toolchain.metadata.lock_file_names {
            let lock_path = deps_root.join(lock_file_name);

            if lock_path.exists()
                && app_context.workspace_config.hasher.optimization == HasherOptimization::Accuracy
                && has_parse_lock
            {
                lock_files.push(
                    toolchain
                        .parse_lock(ParseLockInput {
                            context: app_context.toolchain_registry.create_context(),
                            path: toolchain.to_virtual_path(lock_path),
                            root: toolchain.to_virtual_path(&deps_root),
                        })
                        .await?,
                );
            }
        }

        // Parse and extract project manifest
        for manifest_file_name in &toolchain.metadata.manifest_file_names {
            let manifest_path = project_root.join(manifest_file_name);

            if manifest_path.exists() && has_parse_manifest {
                project_manifests.push(
                    toolchain
                        .parse_manifest(ParseManifestInput {
                            context: app_context.toolchain_registry.create_context(),
                            path: toolchain.to_virtual_path(manifest_path),
                            root: toolchain.to_virtual_path(&deps_root),
                        })
                        .await?,
                );
            }
        }

        // Parse and extract workspace manifest
        if deps_root != project_root {
            for manifest_file_name in &toolchain.metadata.manifest_file_names {
                let manifest_path = deps_root.join(manifest_file_name);

                if manifest_path.exists() && has_parse_manifest {
                    workspace_manifests.push(
                        toolchain
                            .parse_manifest(ParseManifestInput {
                                context: app_context.toolchain_registry.create_context(),
                                path: toolchain.to_virtual_path(manifest_path),
                                root: toolchain.to_virtual_path(&deps_root),
                            })
                            .await?,
                    );
                }
            }
        }
    }

    // Now extract and hash the dependencies
    let inject = apply_toolchain_dependencies_by_manifest(
        project_manifests,
        workspace_manifests,
        lock_files,
        fingerprint,
    );

    Ok(inject)
}

fn apply_toolchain_dependencies_by_manifest(
    project_manifests: Vec<ParseManifestOutput>,
    workspace_manifests: Vec<ParseManifestOutput>,
    locks: Vec<ParseLockOutput>,
    fingerprint: &mut TaskToolchainFingerprint,
) -> bool {
    // Flatten locked deps
    let locked_deps = locks.iter().fold(BTreeMap::default(), |mut map, lock| {
        map.extend(&lock.dependencies);
        map
    });

    // Flatten workspace deps by scope
    let workspace_deps = workspace_manifests.iter().fold(
        BTreeMap::<DependencyScope, BTreeMap<&String, &ManifestDependency>>::default(),
        |mut map, manifest| {
            map.entry(DependencyScope::Peer)
                .or_default()
                .extend(&manifest.peer_dependencies);

            map.entry(DependencyScope::Build)
                .or_default()
                .extend(&manifest.build_dependencies);

            map.entry(DependencyScope::Development)
                .or_default()
                .extend(&manifest.dev_dependencies);

            map.entry(DependencyScope::Production)
                .or_default()
                .extend(&manifest.dependencies);

            map
        },
    );

    let mut inject = false;
    let empty_deps = BTreeMap::new();

    for manifest in project_manifests {
        for (scope, project_deps) in [
            (DependencyScope::Peer, &manifest.peer_dependencies),
            (DependencyScope::Build, &manifest.build_dependencies),
            (DependencyScope::Development, &manifest.dev_dependencies),
            (DependencyScope::Production, &manifest.dependencies),
        ] {
            if apply_toolchain_dependencies_by_scope(
                scope,
                project_deps,
                workspace_deps.get(&scope).unwrap_or(&empty_deps),
                &locked_deps,
                fingerprint,
            ) {
                inject = true;
            }
        }
    }

    inject
}

fn apply_toolchain_dependencies_by_scope(
    scope: DependencyScope,
    project_deps: &BTreeMap<String, ManifestDependency>,
    workspace_deps: &BTreeMap<&String, &ManifestDependency>,
    locked_deps: &BTreeMap<&String, &Vec<LockDependency>>,
    fingerprint: &mut TaskToolchainFingerprint,
) -> bool {
    let mut inject = false;

    for (name, dep) in project_deps {
        let req = if dep.is_inherited() {
            workspace_deps
                .get(name)
                .and_then(|ws_dep| ws_dep.get_version())
        } else {
            dep.get_version()
        };

        // If no version requirement, just skip
        let Some(req) = req else {
            continue;
        };

        // Try and find a resolved version from the lock file
        if let Some(lock_deps) = locked_deps.get(name)
            && let Some(lock_dep) =
                // By exact version first
                lock_deps
                    .iter()
                    .find(|ld| ld.version.as_ref().is_some_and(|v| req == v))
                    .or_else(|| {
                        // Then by matching requirement second
                        lock_deps
                            .iter()
                            .find(|ld| ld.req.as_ref().is_some_and(|r| req == r))
                    })
        {
            // Found, so record a value
            if let Some(hash) = lock_dep
                .hash
                .clone()
                .or_else(|| lock_dep.version.as_ref().map(|v| v.to_string()))
                .or_else(|| lock_dep.meta.clone())
            {
                fingerprint
                    .dependencies
                    .entry(scope)
                    .or_default()
                    .insert(name.to_owned(), hash);
                inject = true;

                continue;
            };
        }

        // None found, so just record the requirement
        fingerprint
            .dependencies
            .entry(scope)
            .or_default()
            .insert(name.to_owned(), req.to_string());
        inject = true;
    }

    inject
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passed(hash: &str) -> TargetState {
        TargetState::Passed(hash.into())
    }

    mod dep_hash_input {
        use super::*;

        #[test]
        fn hash_strategy_uses_dep_hash_when_passed() {
            assert_eq!(
                dep_hash_input(Some(TaskDependencyCacheStrategy::Hash), &passed("abc123")),
                Some("abc123".into())
            );
        }

        #[test]
        fn hash_strategy_marks_passthrough() {
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Hash),
                    &TargetState::Passthrough
                ),
                Some("passthrough".into())
            );
        }

        #[test]
        fn hash_strategy_uses_hash_when_skipped_conditional() {
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Hash),
                    &TargetState::SkippedConditional("cond123".into())
                ),
                Some("cond123".into())
            );
        }

        #[test]
        fn hash_strategy_skips_failed_and_skipped() {
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Hash),
                    &TargetState::Failed
                ),
                None
            );
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Hash),
                    &TargetState::Skipped
                ),
                None
            );
        }

        #[test]
        fn omitted_strategy_behaves_like_hash() {
            // Defensive backstop. All deps should have a resolved cache
            // strategy explicitly set during task expansion time based on
            // whether the dep declares outputs. This test pins the fallback so
            // a hasher reached outside that path (e.g. from a unit test, or a
            // future caller that skips expansion) still produces a stable hash
            // contribution.
            for state in [
                passed("abc"),
                TargetState::Passthrough,
                TargetState::Failed,
                TargetState::Skipped,
                TargetState::SkippedConditional("cond".into()),
            ] {
                assert_eq!(
                    dep_hash_input(None, &state),
                    dep_hash_input(Some(TaskDependencyCacheStrategy::Hash), &state)
                );
            }
        }

        #[test]
        fn ignored_strategy_emits_constant_marker_regardless_of_state() {
            for state in [
                passed("abc"),
                TargetState::Passthrough,
                TargetState::Failed,
                TargetState::Skipped,
                TargetState::SkippedConditional("cond".into()),
            ] {
                assert_eq!(
                    dep_hash_input(Some(TaskDependencyCacheStrategy::Ignored), &state),
                    Some("ignored".into())
                );
            }
        }

        #[test]
        fn outputs_strategy_emits_constant_marker_when_passed() {
            assert_eq!(
                dep_hash_input(Some(TaskDependencyCacheStrategy::Outputs), &passed("abc")),
                Some("outputs".into())
            );
        }

        #[test]
        fn outputs_strategy_distinguishes_passthrough() {
            // Distinct marker so a Passthrough run can't be confused with a
            // Passed run when computing the consumer's hash.
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Outputs),
                    &TargetState::Passthrough
                ),
                Some("outputs-passthrough".into())
            );
        }

        #[test]
        fn outputs_strategy_uses_marker_when_skipped_conditional() {
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Outputs),
                    &TargetState::SkippedConditional("cond123".into())
                ),
                Some("outputs".into())
            );
        }

        #[test]
        fn outputs_strategy_skips_failed_and_skipped() {
            // Mirrors Hash's bail-out so a stale on-disk output from a
            // failed/skipped dep can't cause a spurious cache hit.
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Outputs),
                    &TargetState::Failed
                ),
                None
            );
            assert_eq!(
                dep_hash_input(
                    Some(TaskDependencyCacheStrategy::Outputs),
                    &TargetState::Skipped
                ),
                None
            );
        }

        #[test]
        fn distinct_strategies_yield_distinct_markers_for_passed_state() {
            // Regression guard: each strategy must produce a unique hash
            // contribution so that two deps with different strategies don't
            // collapse to the same cache identity.
            let state = passed("dep-hash");
            let hash = dep_hash_input(Some(TaskDependencyCacheStrategy::Hash), &state).unwrap();
            let ignored =
                dep_hash_input(Some(TaskDependencyCacheStrategy::Ignored), &state).unwrap();
            let outputs =
                dep_hash_input(Some(TaskDependencyCacheStrategy::Outputs), &state).unwrap();

            assert_ne!(hash, ignored);
            assert_ne!(hash, outputs);
            assert_ne!(ignored, outputs);
        }
    }
}
