use crate::task_hasher::TaskHasher;
use miette::IntoDiagnostic;
use moon_action::ActionNode;
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_config::{
    DependencyScope, HasherOptimization, ProjectConfig, TaskDependencyCacheStrategy,
    UnresolvedVersionSpec, VersionSpec,
};
use moon_hash::{ContentHasher, fingerprint};
use moon_pdk_api::{
    HashTaskContentsInput, LocateDependenciesRootInput, LockDependency, ManifestDependency,
    MatchesVersion, ParseLockInput, ParseLockOutput, ParseManifestInput, ParseManifestOutput,
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
            // Only dependencies that must complete before this task can affect
            // its outputs. The others have not ran (cleanup), or may still be
            // running (wait), so their state would make the hash nondeterministic
            if !dep.type_of.is_required_type() {
                continue;
            }

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

    // Try and locate a dependency root
    let Some(deps_workspace) = toolchain
        .locate_dependencies_root(LocateDependenciesRootInput {
            context: app_context.toolchain_registry.create_context(),
            starting_dir: toolchain.to_virtual_path(&project_root),
            toolchain_config: app_context
                .toolchain_registry
                .create_merged_config(&toolchain.id, project_config),
        })
        .await?
    else {
        return Ok(false);
    };

    // Parse and extract locked dependencies
    for lock_file_name in &toolchain.metadata.lock_file_names {
        let lock_path = deps_workspace.root.join(lock_file_name);

        if lock_path.exists()
            && app_context.workspace_config.hasher.optimization == HasherOptimization::Accuracy
        {
            lock_files.push(
                toolchain
                    .parse_lock(ParseLockInput {
                        context: app_context.toolchain_registry.create_context(),
                        path: toolchain.to_virtual_path(lock_path),
                        root: toolchain.to_virtual_path(&deps_workspace.root),
                    })
                    .await?,
            );
        }
    }

    // Parse and extract project manifest
    for manifest_file_name in &toolchain.metadata.manifest_file_names {
        let manifest_path = project_root.join(manifest_file_name);

        if manifest_path.exists() {
            project_manifests.push(
                toolchain
                    .parse_manifest(ParseManifestInput {
                        context: app_context.toolchain_registry.create_context(),
                        path: toolchain.to_virtual_path(manifest_path),
                        root: toolchain.to_virtual_path(&deps_workspace.root),
                    })
                    .await?,
            );
        }
    }

    // Parse and extract workspace manifest, unless the project is at the root,
    // as it's the project's manifest, which is reused below
    let is_root_project = deps_workspace.root == project_root;

    if !is_root_project {
        for manifest_file_name in &toolchain.metadata.manifest_file_names {
            let manifest_path = deps_workspace.root.join(manifest_file_name);

            if manifest_path.exists() {
                workspace_manifests.push(
                    toolchain
                        .parse_manifest(ParseManifestInput {
                            context: app_context.toolchain_registry.create_context(),
                            path: toolchain.to_virtual_path(manifest_path),
                            root: toolchain.to_virtual_path(&deps_workspace.root),
                        })
                        .await?,
                );
            }
        }
    }

    // Now extract and hash the dependencies. A project at the root (like a
    // workspace's root package) inherits versions from its own manifest
    let inject = apply_toolchain_dependencies_by_manifest(
        &project_manifests,
        if is_root_project {
            &project_manifests
        } else {
            &workspace_manifests
        },
        &lock_files,
        fingerprint,
    );

    Ok(inject)
}

fn apply_toolchain_dependencies_by_manifest(
    project_manifests: &[ParseManifestOutput],
    workspace_manifests: &[ParseManifestOutput],
    locks: &[ParseLockOutput],
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

    for manifest in project_manifests {
        for (scope, project_deps) in [
            (DependencyScope::Peer, &manifest.peer_dependencies),
            (DependencyScope::Build, &manifest.build_dependencies),
            (DependencyScope::Development, &manifest.dev_dependencies),
            (DependencyScope::Production, &manifest.dependencies),
        ] {
            if project_deps.is_empty() {
                continue;
            }

            // Inherited versions are defined once for the workspace, whichever
            // scope inherits them (like Cargo's `[workspace.dependencies]`),
            // which plugins return as production dependencies of the workspace
            // manifest. The same scope is only a fallback, as for a root that's
            // also a package, it's that package's own dependencies.
            let inheritable_deps = [DependencyScope::Production, scope]
                .iter()
                .filter_map(|scope| workspace_deps.get(scope))
                .collect::<Vec<_>>();

            if apply_toolchain_dependencies_by_scope(
                scope,
                project_deps,
                &inheritable_deps,
                &locked_deps,
                fingerprint,
            ) {
                inject = true;
            }
        }
    }

    inject
}

/// Returns true if `resolved`, a version taken from a lock file, satisfies the
/// manifest requirement `req`.
///
/// `UnresolvedVersionSpec`'s `PartialEq<VersionSpec>` compares only fully-qualified
/// versions and aliases, so every range and requirement silently fails to match a
/// resolved version. Equality alone therefore matches exact pins only, and the
/// caller falls back to recording the requirement string rather than the resolved
/// version.
fn satisfies_locked_version(req: &UnresolvedVersionSpec, resolved: &VersionSpec) -> bool {
    match (req, resolved.as_version()) {
        (UnresolvedVersionSpec::Range(range), Some(version)) => range.matches(version),
        (UnresolvedVersionSpec::Requirement(req), Some(version)) => req.matches(version),
        _ => req == resolved,
    }
}

/// The value to record for a locked dependency.
fn get_locked_value(lock_dep: &LockDependency) -> Option<String> {
    lock_dep
        .hash
        .clone()
        .or_else(|| lock_dep.version.as_ref().map(|v| v.to_string()))
        .or_else(|| lock_dep.meta.clone())
}

fn apply_toolchain_dependencies_by_scope(
    scope: DependencyScope,
    project_deps: &BTreeMap<String, ManifestDependency>,
    inheritable_deps: &[&BTreeMap<&String, &ManifestDependency>],
    locked_deps: &BTreeMap<&String, &Vec<LockDependency>>,
    fingerprint: &mut TaskToolchainFingerprint,
) -> bool {
    let mut inject = false;

    for (name, dep) in project_deps {
        let inherited = dep.is_inherited();

        // A plugin may resolve an inherited version itself, from the workspace's
        // own definitions, which is preferred, as the workspace manifest's
        // dependencies may also include a root package's own. Otherwise the
        // first workspace dependency with a version, as a workspace root that's
        // also a package may inherit its own workspace dependency
        let req = dep.get_version().or_else(|| {
            if inherited {
                inheritable_deps
                    .iter()
                    .find_map(|deps| deps.get(name).and_then(|ws_dep| ws_dep.get_version()))
            } else {
                None
            }
        });

        let Some(req) = req else {
            // An inherited version that the workspace manifest doesn't resolve
            // is still locked. Like for a workspace root package that inherits
            // its own workspace dependency, whose version a plugin may replace
            // with the inherited marker. Every locked version is recorded, as
            // without a requirement, the one it uses is unknown. Lock files are
            // only parsed when optimizing for accuracy.
            if inherited && let Some(lock_deps) = locked_deps.get(name) {
                let mut values = lock_deps
                    .iter()
                    .filter_map(get_locked_value)
                    .collect::<Vec<_>>();

                if !values.is_empty() {
                    values.sort();
                    values.dedup();

                    fingerprint
                        .dependencies
                        .entry(scope)
                        .or_default()
                        .insert(name.to_owned(), values.join(","));
                    inject = true;
                }
            }

            // Otherwise without a version requirement, just skip
            continue;
        };

        // Try and find a resolved version from the lock file
        if let Some(lock_deps) = locked_deps.get(name)
            && let Some(lock_dep) =
                // By satisfying resolved version first
                lock_deps
                    .iter()
                    .find(|ld| {
                        ld.version
                            .as_ref()
                            .is_some_and(|v| satisfies_locked_version(req, v))
                    })
                    .or_else(|| {
                        // Then by matching requirement second
                        lock_deps
                            .iter()
                            .find(|ld| ld.req.as_ref().is_some_and(|r| req == r))
                    })
        {
            // Found, so record a value
            if let Some(hash) = get_locked_value(lock_dep) {
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

    mod satisfies_locked_version {
        use super::*;

        fn req(value: &str) -> UnresolvedVersionSpec {
            UnresolvedVersionSpec::parse(value).unwrap()
        }

        fn resolved(value: &str) -> VersionSpec {
            VersionSpec::parse(value).unwrap()
        }

        #[test]
        fn tilde_requirement_matches_patch_within_range() {
            assert!(satisfies_locked_version(
                &req("~2.16.9"),
                &resolved("2.16.11")
            ));
        }

        #[test]
        fn tilde_requirement_rejects_version_outside_range() {
            assert!(!satisfies_locked_version(
                &req("~2.16.9"),
                &resolved("2.17.0")
            ));
            assert!(!satisfies_locked_version(
                &req("~2.16.9"),
                &resolved("2.16.8")
            ));
        }

        #[test]
        fn caret_requirement_matches_minor_within_range() {
            assert!(satisfies_locked_version(&req("^1.2.3"), &resolved("1.9.0")));
            assert!(!satisfies_locked_version(
                &req("^1.2.3"),
                &resolved("2.0.0")
            ));
        }

        #[test]
        fn multi_comparator_range_is_bounded_on_both_ends() {
            assert!(matches!(
                req(">=1.2.0 <2.0.0"),
                UnresolvedVersionSpec::Range(_)
            ));
            assert!(satisfies_locked_version(
                &req(">=1.2.0 <2.0.0"),
                &resolved("1.5.0")
            ));
            assert!(!satisfies_locked_version(
                &req(">=1.2.0 <2.0.0"),
                &resolved("2.0.0")
            ));
            assert!(!satisfies_locked_version(
                &req(">=1.2.0 <2.0.0"),
                &resolved("1.1.0")
            ));
        }

        #[test]
        fn exact_version_matches_only_itself() {
            assert!(satisfies_locked_version(
                &req("2.16.9"),
                &resolved("2.16.9")
            ));
            assert!(!satisfies_locked_version(
                &req("2.16.9"),
                &resolved("2.16.11")
            ));
        }

        #[test]
        fn alias_falls_back_to_equality() {
            assert!(satisfies_locked_version(
                &req("latest"),
                &VersionSpec::Alias("latest".into())
            ));
            assert!(!satisfies_locked_version(
                &req("latest"),
                &resolved("1.0.0")
            ));
        }
    }

    mod apply_toolchain_dependencies_by_scope {
        use super::*;

        fn lock_entry(version: &str, hash: &str) -> LockDependency {
            LockDependency {
                hash: Some(hash.into()),
                version: Some(VersionSpec::parse(version).unwrap()),
                ..Default::default()
            }
        }

        // Returns the value recorded for a single production dependency named
        // "dep", declared as `declared` and locked to `lock_entries`.
        fn recorded(declared: &str, lock_entries: Vec<LockDependency>) -> Option<String> {
            let name = "dep".to_owned();
            let mut project_deps = BTreeMap::new();

            project_deps.insert(
                name.clone(),
                ManifestDependency::new(UnresolvedVersionSpec::parse(declared).unwrap()),
            );

            let mut locked_deps = BTreeMap::new();
            locked_deps.insert(&name, &lock_entries);

            let mut fingerprint = TaskToolchainFingerprint {
                toolchain: "test".into(),
                version: None,
                contents: vec![],
                dependencies: BTreeMap::new(),
            };

            apply_toolchain_dependencies_by_scope(
                DependencyScope::Production,
                &project_deps,
                &[],
                &locked_deps,
                &mut fingerprint,
            );

            fingerprint
                .dependencies
                .get(&DependencyScope::Production)
                .and_then(|deps| deps.get("dep"))
                .cloned()
        }

        #[test]
        fn records_lock_hash_for_a_ranged_dependency() {
            assert_eq!(
                recorded("~2.16.9", vec![lock_entry("2.16.11", "sha512-newer")]),
                Some("sha512-newer".into())
            );
        }

        #[test]
        fn ranged_dependency_tracks_the_resolved_version() {
            assert_ne!(
                recorded("~2.16.9", vec![lock_entry("2.16.9", "sha512-older")]),
                recorded("~2.16.9", vec![lock_entry("2.16.11", "sha512-newer")])
            );
        }

        #[test]
        fn records_requirement_when_no_locked_version_satisfies_it() {
            assert_eq!(
                recorded("~2.16.9", vec![lock_entry("3.0.0", "sha512-major")]),
                Some("~2.16.9".into())
            );
        }

        #[test]
        fn records_lock_hash_for_an_exact_dependency() {
            assert_eq!(
                recorded("2.16.9", vec![lock_entry("2.16.9", "sha512-exact")]),
                Some("sha512-exact".into())
            );
        }
    }

    // Versions inherited from the workspace, like Cargo's `workspace = true`,
    // where the plugin returns `[workspace.dependencies]` as production
    // dependencies of the workspace manifest
    mod apply_toolchain_dependencies_by_manifest {
        use super::*;
        use moon_pdk_api::ManifestDependencyConfig;
        use std::path::PathBuf;

        type Recorded = BTreeMap<DependencyScope, BTreeMap<String, String>>;

        fn version(value: &str) -> ManifestDependency {
            ManifestDependency::new(UnresolvedVersionSpec::parse(value).unwrap())
        }

        fn deps<const N: usize>(
            entries: [(&str, ManifestDependency); N],
        ) -> BTreeMap<String, ManifestDependency> {
            entries
                .into_iter()
                .map(|(name, dep)| (name.to_owned(), dep))
                .collect()
        }

        fn lock<const N: usize>(entries: [(&str, &str, &str); N]) -> ParseLockOutput {
            let mut output = ParseLockOutput::default();

            for (name, version, hash) in entries {
                output
                    .dependencies
                    .entry(name.to_owned())
                    .or_default()
                    .push(LockDependency {
                        hash: Some(hash.into()),
                        version: Some(VersionSpec::parse(version).unwrap()),
                        ..Default::default()
                    });
            }

            output
        }

        fn workspace() -> ParseManifestOutput {
            ParseManifestOutput {
                dependencies: deps([("serde", version("^1.0")), ("cc", version("^1.2"))]),
                ..Default::default()
            }
        }

        fn member() -> ParseManifestOutput {
            ParseManifestOutput {
                dev_dependencies: deps([("serde", ManifestDependency::inherited())]),
                build_dependencies: deps([("cc", ManifestDependency::inherited())]),
                ..Default::default()
            }
        }

        fn record(
            project: &[ParseManifestOutput],
            workspace: &[ParseManifestOutput],
            locks: &[ParseLockOutput],
        ) -> (bool, Recorded) {
            let mut fingerprint = TaskToolchainFingerprint {
                toolchain: "test".into(),
                version: None,
                contents: vec![],
                dependencies: BTreeMap::new(),
            };

            let inject = apply_toolchain_dependencies_by_manifest(
                project,
                workspace,
                locks,
                &mut fingerprint,
            );

            (inject, fingerprint.dependencies)
        }

        fn get(recorded: &Recorded, scope: DependencyScope, name: &str) -> Option<String> {
            recorded
                .get(&scope)
                .and_then(|deps| deps.get(name))
                .cloned()
        }

        #[test]
        fn resolves_inherited_dev_and_build_dependencies() {
            let (inject, recorded) = record(&[member()], &[workspace()], &[]);

            assert!(inject);
            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("^1.0".into())
            );
            assert_eq!(
                get(&recorded, DependencyScope::Build, "cc"),
                Some("^1.2".into())
            );
        }

        #[test]
        fn records_the_locked_hash_of_an_inherited_dev_dependency() {
            let (_, recorded) = record(
                &[member()],
                &[workspace()],
                &[lock([("serde", "1.0.200", "sha-serde")])],
            );

            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("sha-serde".into())
            );
        }

        #[test]
        fn prefers_the_workspace_version_over_a_root_package_dependency() {
            let mut root = workspace();
            root.dev_dependencies = deps([("serde", version("^0.9"))]);

            let (_, recorded) = record(&[member()], &[root], &[]);

            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("^1.0".into())
            );
        }

        // The workspace manifest's production dependencies also include a root
        // package's own, which may differ from the workspace's definition
        #[test]
        fn prefers_an_inherited_version_that_the_plugin_resolved() {
            let project = [ParseManifestOutput {
                dev_dependencies: deps([(
                    "serde",
                    ManifestDependency::Config(ManifestDependencyConfig {
                        inherited: true,
                        version: Some(UnresolvedVersionSpec::parse("^1.0").unwrap()),
                        ..Default::default()
                    }),
                )]),
                ..Default::default()
            }];

            let root = ParseManifestOutput {
                dependencies: deps([("serde", version("^0.9"))]),
                ..Default::default()
            };

            let (_, recorded) = record(&project, &[root], &[]);

            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("^1.0".into())
            );
        }

        #[test]
        fn falls_back_to_the_workspace_version_of_the_same_scope() {
            let root = ParseManifestOutput {
                dev_dependencies: deps([("serde", version("^1.0"))]),
                ..Default::default()
            };

            let (_, recorded) = record(&[member()], &[root], &[]);

            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("^1.0".into())
            );
        }

        // The root package's own manifest is also its workspace manifest
        #[test]
        fn resolves_inherited_dependencies_of_a_root_package() {
            let mut root = workspace();
            root.dev_dependencies = deps([("serde", ManifestDependency::inherited())]);

            let root = [root];
            let (_, recorded) = record(&root, &root, &[]);

            assert_eq!(
                get(&recorded, DependencyScope::Development, "serde"),
                Some("^1.0".into())
            );
        }

        // The plugin replaced the workspace version with the inherited marker,
        // as both have the same name in the production dependencies
        #[test]
        fn records_every_locked_hash_of_an_unresolved_inherited_dependency() {
            let root = [ParseManifestOutput {
                dependencies: deps([("serde", ManifestDependency::inherited())]),
                ..Default::default()
            }];

            let (inject, recorded) = record(
                &root,
                &root,
                &[lock([
                    ("serde", "1.0.1", "sha-b"),
                    ("serde", "1.0.0", "sha-a"),
                ])],
            );

            assert!(inject);
            assert_eq!(
                get(&recorded, DependencyScope::Production, "serde"),
                Some("sha-a,sha-b".into())
            );
        }

        #[test]
        fn skips_an_unresolved_inherited_dependency_that_isnt_locked() {
            let root = [ParseManifestOutput {
                dependencies: deps([("serde", ManifestDependency::inherited())]),
                ..Default::default()
            }];

            let (inject, recorded) = record(&root, &root, &[]);

            assert!(!inject);
            assert!(recorded.is_empty());
        }

        #[test]
        fn skips_a_locked_dependency_without_a_version() {
            let project = [ParseManifestOutput {
                dev_dependencies: deps([(
                    "local",
                    ManifestDependency::path(PathBuf::from("../local")),
                )]),
                ..Default::default()
            }];

            let (inject, recorded) = record(
                &project,
                &[workspace()],
                &[lock([("local", "0.1.0", "sha-local")])],
            );

            assert!(!inject);
            assert!(recorded.is_empty());
        }
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
