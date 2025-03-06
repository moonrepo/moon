use crate::create_input_paths;
use moon_common::Id;
use moon_config::*;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

// Turn everything off by default
pub fn get_default_toolchain() -> PartialToolchainConfig {
    PartialToolchainConfig {
        node: Some(PartialNodeConfig {
            version: Some(UnresolvedVersionSpec::parse("18.0.0").unwrap()),
            add_engines_constraint: Some(false),
            dedupe_on_lockfile_change: Some(false),
            infer_tasks_from_scripts: Some(false),
            sync_project_workspace_dependencies: Some(false),
            npm: Some(PartialNpmConfig {
                version: Some(UnresolvedVersionSpec::parse("8.19.0").unwrap()),
                ..PartialNpmConfig::default()
            }),
            ..PartialNodeConfig::default()
        }),
        plugins: Some(FxHashMap::from_iter([(
            Id::raw("typescript"),
            PartialToolchainPluginConfig {
                config: Some(BTreeMap::from_iter([
                    ("createMissingConfig".into(), Value::Bool(false)),
                    ("routeOutDirToCache".into(), Value::Bool(false)),
                    ("syncProjectReferences".into(), Value::Bool(false)),
                    ("syncProjectReferencesToPaths".into(), Value::Bool(false)),
                ])),
                ..PartialToolchainPluginConfig::default()
            },
        )])),
        ..PartialToolchainConfig::default()
    }
}

pub fn get_cases_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("root".try_into().unwrap(), ".".to_owned()),
            ("affected".try_into().unwrap(), "affected".to_owned()),
            ("noAffected".try_into().unwrap(), "no-affected".to_owned()),
            ("base".try_into().unwrap(), "base".to_owned()),
            ("noop".try_into().unwrap(), "noop".to_owned()),
            ("files".try_into().unwrap(), "files".to_owned()),
            ("states".try_into().unwrap(), "states".to_owned()),
            ("taskScript".try_into().unwrap(), "task-script".to_owned()),
            ("taskOs".try_into().unwrap(), "task-os".to_owned()),
            // Runner
            ("interactive".try_into().unwrap(), "interactive".to_owned()),
            ("mutex".try_into().unwrap(), "mutex".to_owned()),
            (
                "passthroughArgs".try_into().unwrap(),
                "passthrough-args".to_owned(),
            ),
            // Project/task deps
            ("depsA".try_into().unwrap(), "deps-a".to_owned()),
            ("depsB".try_into().unwrap(), "deps-b".to_owned()),
            ("depsC".try_into().unwrap(), "deps-c".to_owned()),
            ("dependsOn".try_into().unwrap(), "depends-on".to_owned()),
            ("taskDeps".try_into().unwrap(), "task-deps".to_owned()),
            // Target scopes
            (
                "targetScopeA".try_into().unwrap(),
                "target-scope-a".to_owned(),
            ),
            (
                "targetScopeB".try_into().unwrap(),
                "target-scope-b".to_owned(),
            ),
            (
                "targetScopeC".try_into().unwrap(),
                "target-scope-c".to_owned(),
            ),
            // Outputs
            ("outputs".try_into().unwrap(), "outputs".to_owned()),
            (
                "outputsFiltering".try_into().unwrap(),
                "outputs-filtering".to_owned(),
            ),
            (
                "outputStyles".try_into().unwrap(),
                "output-styles".to_owned(),
            ),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = PartialInheritedTasksConfig {
        tasks: Some(BTreeMap::from_iter([(
            "noop".try_into().unwrap(),
            PartialTaskConfig {
                command: Some(PartialTaskArgs::String("noop".into())),
                ..PartialTaskConfig::default()
            },
        )])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_projects_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("advanced".try_into().unwrap(), "advanced".to_owned()),
            ("basic".try_into().unwrap(), "basic".to_owned()),
            ("emptyConfig".try_into().unwrap(), "empty-config".to_owned()),
            ("noConfig".try_into().unwrap(), "no-config".to_owned()),
            ("metadata".try_into().unwrap(), "metadata".to_owned()),
            ("tasks".try_into().unwrap(), "tasks".to_owned()),
            ("platforms".try_into().unwrap(), "platforms".to_owned()),
            // Deps
            ("foo".try_into().unwrap(), "deps/foo".to_owned()),
            ("bar".try_into().unwrap(), "deps/bar".to_owned()),
            ("baz".try_into().unwrap(), "deps/baz".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = PartialInheritedTasksConfig {
        file_groups: Some(FxHashMap::from_iter([
            (
                "sources".try_into().unwrap(),
                create_input_paths(["src/**/*", "types/**/*"]),
            ),
            (
                "tests".try_into().unwrap(),
                create_input_paths(["tests/**/*"]),
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_project_graph_aliases_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("explicit".try_into().unwrap(), "explicit".to_owned()),
            (
                "explicitAndImplicit".try_into().unwrap(),
                "explicit-and-implicit".to_owned(),
            ),
            ("implicit".try_into().unwrap(), "implicit".to_owned()),
            ("noLang".try_into().unwrap(), "no-lang".to_owned()),
            // Node.js
            ("node".try_into().unwrap(), "node".to_owned()),
            (
                "nodeNameOnly".try_into().unwrap(),
                "node-name-only".to_owned(),
            ),
            (
                "nodeNameScope".try_into().unwrap(),
                "node-name-scope".to_owned(),
            ),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = PartialToolchainConfig {
        node: Some(PartialNodeConfig {
            version: Some(UnresolvedVersionSpec::parse("18.0.0").unwrap()),
            add_engines_constraint: Some(false),
            dedupe_on_lockfile_change: Some(false),
            npm: Some(PartialNpmConfig {
                version: Some(UnresolvedVersionSpec::parse("8.19.0").unwrap()),
                ..PartialNpmConfig::default()
            }),
            ..PartialNodeConfig::default()
        }),
        ..PartialToolchainConfig::default()
    };

    let tasks_config = PartialInheritedTasksConfig::default();

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_tasks_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("basic".try_into().unwrap(), "basic".to_owned()),
            ("buildA".try_into().unwrap(), "build-a".to_owned()),
            ("buildB".try_into().unwrap(), "build-b".to_owned()),
            ("buildC".try_into().unwrap(), "build-c".to_owned()),
            ("chain".try_into().unwrap(), "chain".to_owned()),
            ("cycle".try_into().unwrap(), "cycle".to_owned()),
            ("inheritTags".try_into().unwrap(), "inherit-tags".to_owned()),
            ("inputA".try_into().unwrap(), "input-a".to_owned()),
            ("inputB".try_into().unwrap(), "input-b".to_owned()),
            ("inputC".try_into().unwrap(), "input-c".to_owned()),
            ("inputs".try_into().unwrap(), "inputs".to_owned()),
            (
                "mergeAllStrategies".try_into().unwrap(),
                "merge-all-strategies".to_owned(),
            ),
            ("mergeAppend".try_into().unwrap(), "merge-append".to_owned()),
            (
                "mergePrepend".try_into().unwrap(),
                "merge-prepend".to_owned(),
            ),
            (
                "mergeReplace".try_into().unwrap(),
                "merge-replace".to_owned(),
            ),
            ("noTasks".try_into().unwrap(), "no-tasks".to_owned()),
            ("persistent".try_into().unwrap(), "persistent".to_owned()),
            ("scopeAll".try_into().unwrap(), "scope-all".to_owned()),
            ("scopeDeps".try_into().unwrap(), "scope-deps".to_owned()),
            ("scopeSelf".try_into().unwrap(), "scope-self".to_owned()),
            ("tokens".try_into().unwrap(), "tokens".to_owned()),
            ("expandEnv".try_into().unwrap(), "expand-env".to_owned()),
            (
                "expandEnvProject".try_into().unwrap(),
                "expand-env-project".to_owned(),
            ),
            (
                "expandOutputs".try_into().unwrap(),
                "expand-outputs".to_owned(),
            ),
            ("fileGroups".try_into().unwrap(), "file-groups".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = PartialInheritedTasksConfig {
        file_groups: Some(FxHashMap::from_iter([
            (
                "static".try_into().unwrap(),
                create_input_paths([
                    "file.ts",
                    "dir",
                    "dir/other.tsx",
                    "dir/subdir",
                    "dir/subdir/another.ts",
                ]),
            ),
            (
                "dirs_glob".try_into().unwrap(),
                create_input_paths(["**/*"]),
            ),
            (
                "files_glob".try_into().unwrap(),
                create_input_paths(["**/*.{ts,tsx}"]),
            ),
            (
                "globs".try_into().unwrap(),
                create_input_paths(["**/*.{ts,tsx}", "*.js"]),
            ),
            (
                "no_globs".try_into().unwrap(),
                create_input_paths(["config.js"]),
            ),
        ])),
        tasks: Some(BTreeMap::from_iter([
            (
                "standard".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("cmd".into())),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "withArgs".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("cmd".into())),
                    args: Some(PartialTaskArgs::List(vec![
                        "--foo".into(),
                        "--bar".into(),
                        "baz".into(),
                    ])),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "withInputs".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("cmd".into())),
                    inputs: Some(vec![
                        InputPath::from_str("rel/file.*").unwrap(),
                        InputPath::from_str("/root.*").unwrap(),
                    ]),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "withOutputs".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("cmd".into())),
                    inputs: Some(vec![
                        InputPath::from_str("lib").unwrap(),
                        InputPath::from_str("/build").unwrap(),
                    ]),
                    ..PartialTaskConfig::default()
                },
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

// JAVASCRIPT

pub fn get_bun_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("bun".try_into().unwrap(), "base".to_owned()),
            (
                "packageManager".try_into().unwrap(),
                "package-manager".to_owned(),
            ),
            ("scripts".try_into().unwrap(), "scripts".to_owned()),
            (
                "versionOverride".try_into().unwrap(),
                "version-override".to_owned(),
            ),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let mut toolchain_config = get_default_toolchain();
    toolchain_config.node = None;
    toolchain_config.bun = Some(PartialBunConfig {
        infer_tasks_from_scripts: Some(true),
        version: Some(UnresolvedVersionSpec::parse("1.2.2").unwrap()),
        ..PartialBunConfig::default()
    });

    let tasks_config = PartialInheritedTasksConfig {
        tasks: Some(BTreeMap::from_iter([
            (
                "version".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("bun".into())),
                    args: Some(PartialTaskArgs::String("--version".into())),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "noop".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("noop".into())),
                    ..PartialTaskConfig::default()
                },
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_deno_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("deno".try_into().unwrap(), "base".to_owned()),
            (
                "versionOverride".try_into().unwrap(),
                "version-override".to_owned(),
            ),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let mut toolchain_config = get_default_toolchain();
    toolchain_config.node = None;
    toolchain_config.deno = Some(PartialDenoConfig {
        version: Some(UnresolvedVersionSpec::parse("2.1.9").unwrap()),
        ..PartialDenoConfig::default()
    });

    let tasks_config = PartialInheritedTasksConfig {
        tasks: Some(BTreeMap::from_iter([
            (
                "version".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("deno".into())),
                    args: Some(PartialTaskArgs::String("--version".into())),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "noop".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("noop".into())),
                    ..PartialTaskConfig::default()
                },
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_node_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("node".try_into().unwrap(), "base".to_owned()),
            ("lifecycles".try_into().unwrap(), "lifecycles".to_owned()),
            ("postinstall".try_into().unwrap(), "postinstall".to_owned()),
            (
                "postinstallRecursion".try_into().unwrap(),
                "postinstall-recursion".to_owned(),
            ),
            (
                "versionOverride".try_into().unwrap(),
                "version-override".to_owned(),
            ),
            // Binaries
            ("esbuild".try_into().unwrap(), "esbuild".to_owned()),
            ("swc".try_into().unwrap(), "swc".to_owned()),
            // Project/task deps
            ("depsA".try_into().unwrap(), "deps-a".to_owned()),
            ("depsB".try_into().unwrap(), "deps-b".to_owned()),
            ("depsC".try_into().unwrap(), "deps-c".to_owned()),
            ("depsD".try_into().unwrap(), "deps-d".to_owned()),
            ("dependsOn".try_into().unwrap(), "depends-on".to_owned()),
            (
                "dependsOnScopes".try_into().unwrap(),
                "depends-on-scopes".to_owned(),
            ),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let tasks_config = PartialInheritedTasksConfig {
        tasks: Some(BTreeMap::from_iter([
            (
                "version".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("node".into())),
                    args: Some(PartialTaskArgs::String("--version".into())),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "noop".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("noop".into())),
                    ..PartialTaskConfig::default()
                },
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_python_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([(
            "python".try_into().unwrap(),
            "base".to_owned(),
        )]))),
        ..PartialWorkspaceConfig::default()
    };

    let mut toolchain_config = get_default_toolchain();
    toolchain_config.node = None;
    toolchain_config.python = Some(PartialPythonConfig {
        version: Some(UnresolvedVersionSpec::parse("3.11.10").unwrap()),
        ..PartialPythonConfig::default()
    });

    let tasks_config = PartialInheritedTasksConfig {
        tasks: Some(BTreeMap::from_iter([
            (
                "version".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("python".into())),
                    args: Some(PartialTaskArgs::String("--version".into())),
                    ..PartialTaskConfig::default()
                },
            ),
            (
                "noop".try_into().unwrap(),
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("noop".into())),
                    ..PartialTaskConfig::default()
                },
            ),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_node_depman_fixture_configs(
    depman: &str,
) -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let (mut workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    workspace_config.projects = Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
        (depman.try_into().unwrap(), "base".to_owned()),
        ("other".try_into().unwrap(), "other".to_owned()),
        (
            "notInWorkspace".try_into().unwrap(),
            "not-in-workspace".to_owned(),
        ),
    ])));

    if let Some(node_config) = &mut toolchain_config.node {
        match depman {
            "bun" => {
                node_config.package_manager = Some(NodePackageManager::Bun);
                node_config.bun = Some(PartialBunpmConfig {
                    version: Some(UnresolvedVersionSpec::parse("1.1.19").unwrap()),
                    ..PartialBunpmConfig::default()
                });
            }
            "npm" => {
                node_config.package_manager = Some(NodePackageManager::Npm);
                node_config.npm = Some(PartialNpmConfig {
                    version: Some(UnresolvedVersionSpec::parse("8.0.0").unwrap()),
                    ..PartialNpmConfig::default()
                });
            }
            "pnpm" => {
                node_config.package_manager = Some(NodePackageManager::Pnpm);
                node_config.pnpm = Some(PartialPnpmConfig {
                    version: Some(UnresolvedVersionSpec::parse("7.5.0").unwrap()),
                    ..PartialPnpmConfig::default()
                });
            }
            "yarn" => {
                node_config.package_manager = Some(NodePackageManager::Yarn);
                node_config.yarn = Some(PartialYarnConfig {
                    version: Some(UnresolvedVersionSpec::parse("3.3.0").unwrap()),
                    plugins: Some(vec!["workspace-tools".into()]),
                    ..PartialYarnConfig::default()
                });
            }
            "yarn1" => {
                node_config.package_manager = Some(NodePackageManager::Yarn);
                node_config.yarn = Some(PartialYarnConfig {
                    version: Some(UnresolvedVersionSpec::parse("1.22.0").unwrap()),
                    plugins: Some(vec![]),
                    ..PartialYarnConfig::default()
                });
            }
            _ => {}
        }
    }

    (workspace_config, toolchain_config, tasks_config)
}

pub fn get_typescript_fixture_configs() -> (
    PartialWorkspaceConfig,
    PartialToolchainConfig,
    PartialInheritedTasksConfig,
) {
    let (mut workspace_config, mut toolchain_config, tasks_config) = get_node_fixture_configs();

    workspace_config.projects = Some(PartialWorkspaceProjects::Both(
        PartialWorkspaceProjectsConfig {
            globs: Some(vec!["*".into()]),
            sources: Some(FxHashMap::from_iter([(
                "root".try_into().unwrap(),
                ".".into(),
            )])),
        },
    ));

    if let Some(ts) = &mut toolchain_config
        .plugins
        .get_or_insert_default()
        .get_mut("typescript")
    {
        let ts_config = ts.config.get_or_insert_default();
        ts_config.insert("createMissingConfig".into(), Value::Bool(true));
        ts_config.insert("syncProjectReferences".into(), Value::Bool(true));
    }

    (workspace_config, toolchain_config, tasks_config)
}
