use moon_config::{
    GlobalProjectConfig, NodeConfig, TaskCommandArgs, TaskConfig, ToolchainConfig,
    TypeScriptConfig, WorkspaceConfig, WorkspaceProjects,
};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

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

    // Turn everything off by default
    let toolchain_config = ToolchainConfig {
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
    };

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
