use moon_config::{
    GlobalProjectConfig, NodeConfig, NodePackageManager, NodeProjectAliasFormat, NpmConfig,
    PnpmConfig, TaskCommandArgs, TaskConfig, ToolchainConfig, TypeScriptConfig, WorkspaceConfig,
    WorkspaceProjects, YarnConfig,
};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

// Turn everything off by default
fn get_default_toolchain() -> ToolchainConfig {
    ToolchainConfig {
        node: Some(NodeConfig {
            version: "18.0.0".into(),
            add_engines_constraint: false,
            dedupe_on_lockfile_change: false,
            infer_tasks_from_scripts: false,
            sync_project_workspace_dependencies: false,
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

pub fn get_cases_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("root".to_owned(), ".".to_owned()),
            ("base".to_owned(), "base".to_owned()),
            ("noop".to_owned(), "noop".to_owned()),
            ("files".to_owned(), "files".to_owned()),
            // Runner
            ("passthroughArgs".to_owned(), "passthrough-args".to_owned()),
            // Project/task deps
            ("depsA".to_owned(), "deps-a".to_owned()),
            ("depsB".to_owned(), "deps-b".to_owned()),
            ("depsC".to_owned(), "deps-c".to_owned()),
            ("dependsOn".to_owned(), "depends-on".to_owned()),
            // Target scopes
            ("targetScopeA".to_owned(), "target-scope-a".to_owned()),
            ("targetScopeB".to_owned(), "target-scope-b".to_owned()),
            ("targetScopeC".to_owned(), "target-scope-c".to_owned()),
            // Outputs
            ("outputs".to_owned(), "outputs".to_owned()),
            ("outputStyles".to_owned(), "output-styles".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let projects_config = GlobalProjectConfig {
        tasks: BTreeMap::from_iter([(
            "noop".to_owned(),
            TaskConfig {
                command: Some(TaskCommandArgs::String("noop".into())),
                ..TaskConfig::default()
            },
        )]),
        ..GlobalProjectConfig::default()
    };

    (workspace_config, toolchain_config, projects_config)
}

pub fn get_projects_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("advanced".to_owned(), "advanced".to_owned()),
            ("basic".to_owned(), "basic".to_owned()),
            ("emptyConfig".to_owned(), "empty-config".to_owned()),
            ("noConfig".to_owned(), "no-config".to_owned()),
            ("tasks".to_owned(), "tasks".to_owned()),
            // Deps
            ("foo".to_owned(), "deps/foo".to_owned()),
            ("bar".to_owned(), "deps/bar".to_owned()),
            ("baz".to_owned(), "deps/baz".to_owned()),
            // Langs
            ("js".to_owned(), "langs/js".to_owned()),
            ("ts".to_owned(), "langs/ts".to_owned()),
            ("bash".to_owned(), "langs/bash".to_owned()),
            ("platforms".to_owned(), "platforms".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let projects_config = GlobalProjectConfig {
        file_groups: FxHashMap::from_iter([
            (
                "sources".into(),
                vec!["src/**/*".into(), "types/**/*".into()],
            ),
            ("tests".into(), vec!["tests/**/*".into()]),
        ]),
        ..GlobalProjectConfig::default()
    };

    (workspace_config, toolchain_config, projects_config)
}

pub fn get_project_graph_aliases_fixture_configs(
) -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("explicit".to_owned(), "explicit".to_owned()),
            (
                "explicitAndImplicit".to_owned(),
                "explicit-and-implicit".to_owned(),
            ),
            ("implicit".to_owned(), "implicit".to_owned()),
            ("noLang".to_owned(), "no-lang".to_owned()),
            // Node.js
            ("node".to_owned(), "node".to_owned()),
            ("nodeNameOnly".to_owned(), "node-name-only".to_owned()),
            ("nodeNameScope".to_owned(), "node-name-scope".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = ToolchainConfig {
        node: Some(NodeConfig {
            version: "18.0.0".into(),
            add_engines_constraint: false,
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
            dedupe_on_lockfile_change: false,
            ..NodeConfig::default()
        }),
        ..ToolchainConfig::default()
    };

    let projects_config = GlobalProjectConfig::default();

    (workspace_config, toolchain_config, projects_config)
}

pub fn get_tasks_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("basic".to_owned(), "basic".to_owned()),
            ("buildA".to_owned(), "build-a".to_owned()),
            ("buildB".to_owned(), "build-b".to_owned()),
            ("buildC".to_owned(), "build-c".to_owned()),
            ("chain".to_owned(), "chain".to_owned()),
            ("cycle".to_owned(), "cycle".to_owned()),
            ("inputA".to_owned(), "input-a".to_owned()),
            ("inputB".to_owned(), "input-b".to_owned()),
            ("inputC".to_owned(), "input-c".to_owned()),
            (
                "mergeAllStrategies".to_owned(),
                "merge-all-strategies".to_owned(),
            ),
            ("mergeAppend".to_owned(), "merge-append".to_owned()),
            ("mergePrepend".to_owned(), "merge-prepend".to_owned()),
            ("mergeReplace".to_owned(), "merge-replace".to_owned()),
            ("noTasks".to_owned(), "no-tasks".to_owned()),
            ("scopeAll".to_owned(), "scope-all".to_owned()),
            ("scopeDeps".to_owned(), "scope-deps".to_owned()),
            ("scopeSelf".to_owned(), "scope-self".to_owned()),
            ("tokens".to_owned(), "tokens".to_owned()),
            ("expandEnv".to_owned(), "expand-env".to_owned()),
            ("expandOutputs".to_owned(), "expand-outputs".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let projects_config = GlobalProjectConfig {
        file_groups: FxHashMap::from_iter([
            (
                "static".into(),
                vec![
                    "file.ts".into(),
                    "dir".into(),
                    "dir/other.tsx".into(),
                    "dir/subdir".into(),
                    "dir/subdir/another.ts".into(),
                ],
            ),
            ("dirs_glob".into(), vec!["**/*".into()]),
            ("files_glob".into(), vec!["**/*.{ts,tsx}".into()]),
            ("globs".into(), vec!["**/*.{ts,tsx}".into(), "*.js".into()]),
            ("no_globs".into(), vec!["config.js".into()]),
        ]),
        tasks: BTreeMap::from_iter([
            (
                "standard".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("cmd".into())),
                    ..TaskConfig::default()
                },
            ),
            (
                "withArgs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("cmd".into())),
                    args: Some(TaskCommandArgs::Sequence(vec![
                        "--foo".into(),
                        "--bar".into(),
                        "baz".into(),
                    ])),
                    ..TaskConfig::default()
                },
            ),
            (
                "withInputs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("cmd".into())),
                    inputs: Some(vec!["rel/file.*".into(), "/root.*".into()]),
                    ..TaskConfig::default()
                },
            ),
            (
                "withOutputs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("cmd".into())),
                    inputs: Some(vec!["lib".into(), "/build".into()]),
                    ..TaskConfig::default()
                },
            ),
        ]),
        ..GlobalProjectConfig::default()
    };

    (workspace_config, toolchain_config, projects_config)
}

// NODE.JS

pub fn get_node_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let workspace_config = WorkspaceConfig {
        projects: WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("node".to_owned(), "base".to_owned()),
            ("lifecycles".to_owned(), "lifecycles".to_owned()),
            ("versionOverride".to_owned(), "version-override".to_owned()),
            // Binaries
            ("esbuild".to_owned(), "esbuild".to_owned()),
            ("swc".to_owned(), "swc".to_owned()),
            // Project/task deps
            ("depsA".to_owned(), "deps-a".to_owned()),
            ("depsB".to_owned(), "deps-b".to_owned()),
            ("depsC".to_owned(), "deps-c".to_owned()),
            ("depsD".to_owned(), "deps-d".to_owned()),
            ("dependsOn".to_owned(), "depends-on".to_owned()),
            ("dependsOnScopes".to_owned(), "depends-on-scopes".to_owned()),
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let projects_config = GlobalProjectConfig {
        tasks: BTreeMap::from_iter([
            (
                "version".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("node".into())),
                    args: Some(TaskCommandArgs::String("--version".into())),
                    ..TaskConfig::default()
                },
            ),
            (
                "noop".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("noop".into())),
                    ..TaskConfig::default()
                },
            ),
        ]),
        ..GlobalProjectConfig::default()
    };

    (workspace_config, toolchain_config, projects_config)
}

pub fn get_node_depman_fixture_configs(
    depman: &str,
) -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let (mut workspace_config, mut toolchain_config, projects_config) = get_node_fixture_configs();

    workspace_config.projects = WorkspaceProjects::Sources(FxHashMap::from_iter([
        (depman.to_owned(), "base".to_owned()),
        ("other".to_owned(), "other".to_owned()),
        ("notInWorkspace".to_owned(), "not-in-workspace".to_owned()),
    ]));

    if let Some(node_config) = &mut toolchain_config.node {
        match depman {
            "npm" => {
                node_config.package_manager = NodePackageManager::Npm;
                node_config.npm = NpmConfig {
                    version: "8.0.0".into(),
                };
            }
            "pnpm" => {
                node_config.package_manager = NodePackageManager::Pnpm;
                node_config.pnpm = Some(PnpmConfig {
                    version: "7.5.0".into(),
                });
            }
            "yarn" => {
                node_config.package_manager = NodePackageManager::Yarn;
                node_config.yarn = Some(YarnConfig {
                    version: "3.3.0".into(),
                    plugins: Some(vec!["workspace-tools".into()]),
                });
            }
            "yarn1" => {
                node_config.package_manager = NodePackageManager::Yarn;
                node_config.yarn = Some(YarnConfig {
                    version: "1.22.0".into(),
                    plugins: None,
                });
            }
            _ => {}
        }
    }

    (workspace_config, toolchain_config, projects_config)
}

pub fn get_typescript_fixture_configs() -> (WorkspaceConfig, ToolchainConfig, GlobalProjectConfig) {
    let (mut workspace_config, mut toolchain_config, projects_config) = get_node_fixture_configs();

    workspace_config.projects = WorkspaceProjects::Globs(vec!["*".into()]);

    if let Some(ts_config) = &mut toolchain_config.typescript {
        ts_config.create_missing_config = true;
        ts_config.sync_project_references = true;
    }

    (workspace_config, toolchain_config, projects_config)
}
