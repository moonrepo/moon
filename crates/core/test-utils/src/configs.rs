use crate::create_portable_paths;
use moon_config2::{
    InheritedTasksConfig, NodeConfig, NodePackageManager, NpmConfig, PnpmConfig, TaskCommandArgs,
    TaskConfig, ToolchainConfig, TypeScriptConfig, WorkspaceConfig, WorkspaceProjects, YarnConfig,
};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

// Turn everything off by default
pub fn get_default_toolchain() -> ToolchainConfig {
    ToolchainConfig {
        node: Some(NodeConfig {
            version: Some("18.0.0".into()),
            add_engines_constraint: false,
            dedupe_on_lockfile_change: false,
            infer_tasks_from_scripts: false,
            sync_project_workspace_dependencies: false,
            npm: NpmConfig {
                version: Some("8.19.0".into()),
            },
            ..NodeConfig::default()
        }),
        typescript: Some(TypeScriptConfig {
            create_missing_config: false,
            route_out_dir_to_cache: false,
            sync_project_references: false,
            sync_project_references_to_paths: false,
            ..TypeScriptConfig::default()
        }),
        ..ToolchainConfig::default()
    }
}

pub fn get_cases_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("root".into(), ".".to_owned()),
            ("affected".into(), "affected".to_owned()),
            ("base".into(), "base".to_owned()),
            ("noop".into(), "noop".to_owned()),
            ("files".into(), "files".to_owned()),
            // Runner
            ("interactive".into(), "interactive".to_owned()),
            ("passthroughArgs".into(), "passthrough-args".to_owned()),
            // Project/task deps
            ("depsA".into(), "deps-a".to_owned()),
            ("depsB".into(), "deps-b".to_owned()),
            ("depsC".into(), "deps-c".to_owned()),
            ("dependsOn".into(), "depends-on".to_owned()),
            // Target scopes
            ("targetScopeA".into(), "target-scope-a".to_owned()),
            ("targetScopeB".into(), "target-scope-b".to_owned()),
            ("targetScopeC".into(), "target-scope-c".to_owned()),
            // Outputs
            ("outputs".into(), "outputs".to_owned()),
            ("outputsFiltering".into(), "outputs-filtering".to_owned()),
            ("outputStyles".into(), "output-styles".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = InheritedTasksConfig {
        tasks: BTreeMap::from_iter([(
            "noop".into(),
            TaskConfig {
                command: TaskCommandArgs::String("noop".into()),
                ..TaskConfig::default()
            },
        )]),
        ..InheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_projects_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("advanced".into(), "advanced".to_owned()),
            ("basic".into(), "basic".to_owned()),
            ("emptyConfig".into(), "empty-config".to_owned()),
            ("noConfig".into(), "no-config".to_owned()),
            ("tasks".into(), "tasks".to_owned()),
            ("platforms".into(), "platforms".to_owned()),
            // Deps
            ("foo".into(), "deps/foo".to_owned()),
            ("bar".into(), "deps/bar".to_owned()),
            ("baz".into(), "deps/baz".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = InheritedTasksConfig {
        file_groups: FxHashMap::from_iter([
            (
                "sources".into(),
                create_portable_paths(["src/**/*", "types/**/*"]),
            ),
            ("tests".into(), create_portable_paths(["tests/**/*"])),
        ]),
        ..InheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_project_graph_aliases_fixture_configs(
) -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("explicit".into(), "explicit".to_owned()),
            (
                "explicitAndImplicit".into(),
                "explicit-and-implicit".to_owned(),
            ),
            ("implicit".into(), "implicit".to_owned()),
            ("noLang".into(), "no-lang".to_owned()),
            // Node.js
            ("node".into(), "node".to_owned()),
            ("nodeNameOnly".into(), "node-name-only".to_owned()),
            ("nodeNameScope".into(), "node-name-scope".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = ToolchainConfig {
        node: Some(NodeConfig {
            version: Some("18.0.0".into()),
            add_engines_constraint: false,
            dedupe_on_lockfile_change: false,
            npm: NpmConfig {
                version: Some("8.19.0".into()),
            },
            ..NodeConfig::default()
        }),
        ..ToolchainConfig::default()
    };

    let tasks_config = InheritedTasksConfig::default();

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_tasks_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("basic".into(), "basic".to_owned()),
            ("buildA".into(), "build-a".to_owned()),
            ("buildB".into(), "build-b".to_owned()),
            ("buildC".into(), "build-c".to_owned()),
            ("chain".into(), "chain".to_owned()),
            ("cycle".into(), "cycle".to_owned()),
            ("inheritTags".into(), "inherit-tags".to_owned()),
            ("inputA".into(), "input-a".to_owned()),
            ("inputB".into(), "input-b".to_owned()),
            ("inputC".into(), "input-c".to_owned()),
            ("inputs".into(), "inputs".to_owned()),
            (
                "mergeAllStrategies".into(),
                "merge-all-strategies".to_owned(),
            ),
            ("mergeAppend".into(), "merge-append".to_owned()),
            ("mergePrepend".into(), "merge-prepend".to_owned()),
            ("mergeReplace".into(), "merge-replace".to_owned()),
            ("noTasks".into(), "no-tasks".to_owned()),
            ("persistent".into(), "persistent".to_owned()),
            ("scopeAll".into(), "scope-all".to_owned()),
            ("scopeDeps".into(), "scope-deps".to_owned()),
            ("scopeSelf".into(), "scope-self".to_owned()),
            ("tokens".into(), "tokens".to_owned()),
            ("expandEnv".into(), "expand-env".to_owned()),
            ("expandEnvProject".into(), "expand-env-project".to_owned()),
            ("expandOutputs".into(), "expand-outputs".to_owned()),
            ("fileGroups".into(), "file-groups".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = InheritedTasksConfig {
        file_groups: FxHashMap::from_iter([
            (
                "static".into(),
                create_portable_paths([
                    "file.ts",
                    "dir",
                    "dir/other.tsx",
                    "dir/subdir",
                    "dir/subdir/another.ts",
                ]),
            ),
            ("dirs_glob".into(), create_portable_paths(["**/*"])),
            (
                "files_glob".into(),
                create_portable_paths(["**/*.{ts,tsx}"]),
            ),
            (
                "globs".into(),
                create_portable_paths(["**/*.{ts,tsx}", "*.js"]),
            ),
            ("no_globs".into(), create_portable_paths(["config.js"])),
        ]),
        tasks: BTreeMap::from_iter([
            (
                "standard".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("cmd".into()),
                    ..TaskConfig::default()
                },
            ),
            (
                "withArgs".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("cmd".into()),
                    args: TaskCommandArgs::Sequence(vec![
                        "--foo".into(),
                        "--bar".into(),
                        "baz".into(),
                    ]),
                    ..TaskConfig::default()
                },
            ),
            (
                "withInputs".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("cmd".into()),
                    inputs: vec!["rel/file.*".into(), "/root.*".into()],
                    ..TaskConfig::default()
                },
            ),
            (
                "withOutputs".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("cmd".into()),
                    inputs: vec!["lib".into(), "/build".into()],
                    ..TaskConfig::default()
                },
            ),
        ]),
        ..InheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

// NODE.JS

pub fn get_node_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("node".into(), "base".to_owned()),
            ("lifecycles".into(), "lifecycles".to_owned()),
            (
                "postinstallRecursion".into(),
                "postinstall-recursion".to_owned(),
            ),
            ("versionOverride".into(), "version-override".to_owned()),
            // Binaries
            ("esbuild".into(), "esbuild".to_owned()),
            ("swc".into(), "swc".to_owned()),
            // Project/task deps
            ("depsA".into(), "deps-a".to_owned()),
            ("depsB".into(), "deps-b".to_owned()),
            ("depsC".into(), "deps-c".to_owned()),
            ("depsD".into(), "deps-d".to_owned()),
            ("dependsOn".into(), "depends-on".to_owned()),
            ("dependsOnScopes".into(), "depends-on-scopes".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = InheritedTasksConfig {
        tasks: BTreeMap::from_iter([
            (
                "version".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("node".into()),
                    args: TaskCommandArgs::String("--version".into()),
                    ..TaskConfig::default()
                },
            ),
            (
                "noop".into(),
                TaskConfig {
                    command: TaskCommandArgs::String("noop".into()),
                    ..TaskConfig::default()
                },
            ),
        ]),
        ..InheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_node_depman_fixture_configs(
    depman: &str,
) -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig) {
    let (mut workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    workspace_config.projects = WorkspaceProjects::Sources(FxHashMap::from_iter([
        (depman.into(), "base".to_owned()),
        ("other".into(), "other".to_owned()),
        ("notInWorkspace".into(), "not-in-workspace".to_owned()),
    ]));

    if let Some(node_config) = &mut toolchain_config.node {
        match depman {
            "npm" => {
                node_config.package_manager = NodePackageManager::Npm;
                node_config.npm = NpmConfig {
                    version: Some("8.0.0".into()),
                };
            }
            "pnpm" => {
                node_config.package_manager = NodePackageManager::Pnpm;
                node_config.pnpm = Some(PnpmConfig {
                    version: Some("7.5.0".into()),
                });
            }
            "yarn" => {
                node_config.package_manager = NodePackageManager::Yarn;
                node_config.yarn = Some(YarnConfig {
                    version: Some("3.3.0".into()),
                    plugins: vec!["workspace-tools".into()],
                });
            }
            "yarn1" => {
                node_config.package_manager = NodePackageManager::Yarn;
                node_config.yarn = Some(YarnConfig {
                    version: Some("1.22.0".into()),
                    plugins: vec![],
                });
            }
            _ => {}
        }
    }

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_typescript_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, InheritedTasksConfig)
{
    let (mut workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    workspace_config.projects = WorkspaceProjects::Both {
        globs: vec!["*".into()],
        sources: FxHashMap::from_iter([("root".into(), ".".into())]),
    };

    if let Some(ts_config) = &mut toolchain_config.typescript {
        ts_config.create_missing_config = true;
        ts_config.sync_project_references = true;
    }

    (workspace_config, toolchain_config, tasks_config)
}
