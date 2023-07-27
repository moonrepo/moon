use moon::{generate_project_graph, load_workspace_from};
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, PartialConstraintsConfig,
    PartialNodeConfig, PartialRustConfig, PartialToolchainConfig, PartialWorkspaceConfig,
    PartialWorkspaceProjects, PartialWorkspaceProjectsConfig,
};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, get_project_graph_aliases_fixture_configs, Sandbox,
};
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;



#[tokio::test]
async fn can_generate_with_deps_cycles() {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".into(), "a".to_owned()),
            ("b".into(), "b".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox =
        create_sandbox_with_config("project-graph/cycle", Some(workspace_config), None, None);

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    assert_eq!(
        graph.sources,
        FxHashMap::from_iter([("a".into(), "a".to_owned()), ("b".into(), "b".to_owned()),])
    );

    assert_eq!(
        graph.get("a").unwrap().get_dependency_ids(),
        vec![&Id::raw("b")]
    );
    assert_eq!(
        graph.get("b").unwrap().get_dependency_ids(),
        vec![&Id::raw("a")]
    );
}

mod caching {
    use super::*;
    use moon_cache::ProjectsState;

    #[tokio::test]
    async fn caches_and_hashes_projects_state() {
        let (_, sandbox) = get_dependencies_graph(true).await;
        let state_path = sandbox.path().join(".moon/cache/states/projects.json");
        let graph_path = sandbox.path().join(".moon/cache/states/projectGraph.json");

        assert!(state_path.exists());
        assert!(graph_path.exists());

        let state = ProjectsState::load(state_path).unwrap();

        assert_eq!(
            state.last_hash,
            "7ea65b6c65b3c9c3f24d6cde0215268c249686eedde0b689b5085e4c116750ed"
        );
        assert_eq!(
            state.projects,
            FxHashMap::from_iter([
                ("a".into(), "a".into()),
                ("b".into(), "b".into()),
                ("c".into(), "c".into()),
                ("d".into(), "d".into()),
            ])
        );

        assert!(sandbox
            .path()
            .join(".moon/cache/hashes")
            .join(format!("{}.json", state.last_hash))
            .exists());
    }

    #[tokio::test]
    async fn doesnt_cache_if_no_vcs() {
        let (_, sandbox) = get_dependencies_graph(false).await;
        sandbox.debug_files();
        let state_path = sandbox.path().join(".moon/cache/states/projects.json");
        let graph_path = sandbox.path().join(".moon/cache/states/projectGraph.json");

        assert!(state_path.exists());
        assert!(!graph_path.exists());

        let state = ProjectsState::load(state_path).unwrap();

        assert_eq!(state.last_hash, "");
    }
}

mod globs {
    use super::*;

    #[tokio::test]
    async fn ignores_moon_dot_folder() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Globs(string_vec!["*"])),
            ..PartialWorkspaceConfig::default()
        };

        // Use git so we can test against the .git folder
        let sandbox =
            create_sandbox_with_config("project-graph/langs", Some(workspace_config), None, None);
        sandbox.enable_git();
        sandbox.create_file(".foo/moon.yml", "{}");
        sandbox.create_file("node_modules/moon/package.json", "{}");

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        assert_eq!(graph.sources.len(), 21);
        assert!(graph.sources.contains_key(&Id::raw(".foo")));
        assert!(!graph.sources.contains_key(&Id::raw(".git")));
        assert!(!graph.sources.contains_key(&Id::raw(".moon")));
        assert!(!graph.sources.contains_key(&Id::raw("node_modules")));
    }

    #[tokio::test]
    async fn filters_ignored_sources() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Globs(string_vec!["*"])),
            ..PartialWorkspaceConfig::default()
        };

        // Use git so we can test against the .git folder
        let sandbox =
            create_sandbox_with_config("project-graph/langs", Some(workspace_config), None, None);
        sandbox.enable_git();
        sandbox.create_file(".gitignore", "*-config");

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        assert_eq!(graph.sources.len(), 12);
        assert!(graph.sources.contains_key(&Id::raw("deno")));
        assert!(!graph.sources.contains_key(&Id::raw("deno-config")));
        assert!(graph.sources.contains_key(&Id::raw("python")));
        assert!(!graph.sources.contains_key(&Id::raw("python-config")));
        assert!(graph.sources.contains_key(&Id::raw("ts")));
        assert!(!graph.sources.contains_key(&Id::raw("ts-config")));
    }

    #[tokio::test]
    async fn supports_all_id_formats() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(PartialWorkspaceProjects::Globs(string_vec!["*"])),
            ..PartialWorkspaceConfig::default()
        };

        let sandbox =
            create_sandbox_with_config("project-graph/ids", Some(workspace_config), None, None);

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let graph = generate_project_graph(&mut workspace).await.unwrap();

        assert_eq!(
            graph.sources,
            FxHashMap::from_iter([
                ("camelCase".into(), "camelCase".to_owned()),
                ("Capital".into(), "Capital".to_owned()),
                ("kebab-case".into(), "kebab-case".to_owned()),
                ("PascalCase".into(), "PascalCase".to_owned()),
                ("snake_case".into(), "snake_case".to_owned()),
                ("With_nums-123".into(), "With_nums-123".to_owned())
            ])
        );
    }
}
