use moon_config::{
    GlobalProjectConfig, NodeConfig, NodeProjectAliasFormat, TaskCommandArgs, TaskConfig,
    ToolchainConfig, TypeScriptConfig, WorkspaceConfig, WorkspaceProjects,
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
        tasks: BTreeMap::from_iter([(
            "version".to_owned(),
            TaskConfig {
                command: Some(TaskCommandArgs::String("node --version".into())),
                ..TaskConfig::default()
            },
        )]),
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
            alias_package_names: Some(NodeProjectAliasFormat::NameAndScope),
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
        ])),
        ..WorkspaceConfig::default()
    };

    let toolchain_config = get_default_toolchain();

    let projects_config = GlobalProjectConfig {
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
