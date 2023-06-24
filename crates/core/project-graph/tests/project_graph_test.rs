use moon::{generate_project_graph, load_workspace_from};
use moon_common::Id;
use moon_config::{
    DependencyConfig, DependencyScope, DependencySource, PartialConstraintsConfig,
    PartialNodeConfig, PartialRustConfig, PartialToolchainConfig, PartialWorkspaceConfig,
    WorkspaceProjects,
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

pub fn append_file<P: AsRef<Path>>(path: P, data: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(path.as_ref())
        .unwrap();

    writeln!(file, "\n\n{data}").unwrap();
}

async fn get_aliases_graph() -> (ProjectGraph, Sandbox) {
    let (workspace_config, toolchain_config, tasks_config) =
        get_project_graph_aliases_fixture_configs();

    let sandbox = create_sandbox_with_config(
        "project-graph/aliases",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_dependencies_graph(enable_git: bool) -> (ProjectGraph, Sandbox) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".into(), "a".to_owned()),
            ("b".into(), "b".to_owned()),
            ("c".into(), "c".to_owned()),
            ("d".into(), "d".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependencies",
        Some(workspace_config),
        None,
        None,
    );

    if enable_git {
        sandbox.enable_git();
    }

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_dependents_graph() -> (ProjectGraph, Sandbox) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Sources(FxHashMap::from_iter([
            ("a".into(), "a".to_owned()),
            ("b".into(), "b".to_owned()),
            ("c".into(), "c".to_owned()),
            ("d".into(), "d".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/dependents",
        Some(workspace_config),
        None,
        None,
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_tag_constraints_graph<F>(setup: F) -> (ProjectGraph, Sandbox)
where
    F: FnOnce(&Sandbox),
{
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Globs(vec!["*".into()])),
        constraints: Some(PartialConstraintsConfig {
            tag_relationships: Some(FxHashMap::from_iter([
                (
                    "warrior".into(),
                    vec![Id::raw("barbarian"), Id::raw("paladin"), Id::raw("druid")],
                ),
                (
                    "mage".into(),
                    vec![Id::raw("wizard"), Id::raw("sorcerer"), Id::raw("druid")],
                ),
            ])),
            ..PartialConstraintsConfig::default()
        }),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/tag-constraints",
        Some(workspace_config),
        None,
        None,
    );

    setup(&sandbox);

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_type_constraints_graph<F>(setup: F) -> (ProjectGraph, Sandbox)
where
    F: FnOnce(&Sandbox),
{
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Globs(vec!["*".into()])),
        constraints: Some(PartialConstraintsConfig {
            enforce_project_type_relationships: Some(true),
            ..PartialConstraintsConfig::default()
        }),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/type-constraints",
        Some(workspace_config),
        None,
        None,
    );

    setup(&sandbox);

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

async fn get_queries_graph() -> (ProjectGraph, Sandbox) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Globs(vec!["*".into()])),
        ..PartialWorkspaceConfig::default()
    };

    let toolchain_config = PartialToolchainConfig {
        node: Some(PartialNodeConfig::default()),
        rust: Some(PartialRustConfig::default()),
        ..PartialToolchainConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "project-graph/query",
        Some(workspace_config),
        Some(toolchain_config),
        None,
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    (graph, sandbox)
}

#[tokio::test]
async fn can_use_map_and_globs_setting() {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Both {
            globs: string_vec!["deps/*"],
            sources: FxHashMap::from_iter([
                ("basic".into(), "basic".to_owned()),
                ("noConfig".into(), "no-config".to_owned()),
            ]),
        }),
        ..PartialWorkspaceConfig::default()
    };

    let sandbox = create_sandbox_with_config("projects", Some(workspace_config), None, None);

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let graph = generate_project_graph(&mut workspace).await.unwrap();

    assert_eq!(
        graph.sources,
        FxHashMap::from_iter([
            ("noConfig".into(), "no-config".to_owned()),
            ("bar".into(), "deps/bar".to_owned()),
            ("basic".into(), "basic".to_owned()),
            ("baz".into(), "deps/baz".to_owned()),
            ("foo".into(), "deps/foo".to_owned()),
        ])
    );
}

#[tokio::test]
async fn can_generate_with_deps_cycles() {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(WorkspaceProjects::Sources(FxHashMap::from_iter([
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

        assert_eq!(state.globs, string_vec![]);
        assert_eq!(state.last_glob_time, 0);
        assert_eq!(
            state.last_hash,
            "7ea65b6c65b3c9c3f24d6cde0215268c249686eedde0b689b5085e4c116750ed"
        );
        assert_eq!(
            state.projects,
            FxHashMap::from_iter([
                ("a".into(), "a".to_string()),
                ("b".into(), "b".to_string()),
                ("c".into(), "c".to_string()),
                ("d".into(), "d".to_string()),
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
            projects: Some(WorkspaceProjects::Globs(string_vec!["*"])),
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
            projects: Some(WorkspaceProjects::Globs(string_vec!["*"])),
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
            projects: Some(WorkspaceProjects::Globs(string_vec!["*"])),
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

mod get_dependencies_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let (graph, _sandbox) = get_dependencies_graph(false).await;

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

        assert_eq!(graph.get_dependencies_of(a).unwrap(), string_vec!["b"]);
        assert_eq!(graph.get_dependencies_of(b).unwrap(), string_vec!["c"]);
        assert_eq!(graph.get_dependencies_of(c).unwrap(), string_vec![]);
        assert_eq!(
            graph.get_dependencies_of(d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod get_dependents_of {
    use super::*;

    #[tokio::test]
    async fn returns_dep_list() {
        let (graph, _sandbox) = get_dependents_graph().await;

        let a = graph.get("a").unwrap();
        let b = graph.get("b").unwrap();
        let c = graph.get("c").unwrap();
        let d = graph.get("d").unwrap();

        assert_eq!(graph.get_dependents_of(a).unwrap(), string_vec![]);
        assert_eq!(graph.get_dependents_of(b).unwrap(), string_vec!["a"]);
        assert_eq!(graph.get_dependents_of(c).unwrap(), string_vec!["b"]);
        assert_eq!(
            graph.get_dependents_of(d).unwrap(),
            string_vec!["a", "b", "c"]
        );
    }
}

mod to_dot {
    use super::*;
    use moon::build_project_graph;

    #[tokio::test]
    async fn renders_tree() {
        let (graph, _sandbox) = get_dependencies_graph(false).await;

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    async fn renders_partial_tree() {
        let workspace_config = PartialWorkspaceConfig {
            projects: Some(WorkspaceProjects::Sources(FxHashMap::from_iter([
                ("a".into(), "a".to_owned()),
                ("b".into(), "b".to_owned()),
                ("c".into(), "c".to_owned()),
                ("d".into(), "d".to_owned()),
            ]))),
            ..PartialWorkspaceConfig::default()
        };

        let sandbox = create_sandbox_with_config(
            "project-graph/dependencies",
            Some(workspace_config),
            None,
            None,
        );

        let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
        let mut graph = build_project_graph(&mut workspace).await.unwrap();

        graph.load("b").unwrap();

        let graph = graph.build().unwrap();

        assert_snapshot!(graph.to_dot());
    }
}

mod implicit_explicit_deps {
    use super::*;

    #[tokio::test]
    async fn loads_implicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("implicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".into(),
                    DependencyConfig {
                        id: "nodeNameScope".into(),
                        scope: DependencyScope::Development,
                        source: DependencySource::Implicit,
                        via: Some("@scope/pkg-foo".to_string())
                    }
                ),
                (
                    "node".into(),
                    DependencyConfig {
                        id: "node".into(),
                        scope: DependencyScope::Production,
                        source: DependencySource::Implicit,
                        via: Some("project-graph-aliases-node".to_string())
                    }
                )
            ])
        );
    }

    #[tokio::test]
    async fn loads_explicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("explicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".into(),
                    DependencyConfig {
                        id: "nodeNameScope".into(),
                        scope: DependencyScope::Production,
                        source: DependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "node".into(),
                    DependencyConfig {
                        id: "node".into(),
                        scope: DependencyScope::Development,
                        source: DependencySource::Explicit,
                        via: None
                    }
                )
            ])
        );
    }

    #[tokio::test]
    async fn loads_explicit_and_implicit() {
        let (graph, _sandbox) = get_aliases_graph().await;

        let project = graph.get("explicitAndImplicit").unwrap();

        assert_eq!(
            project.dependencies,
            FxHashMap::from_iter([
                (
                    "nodeNameScope".into(),
                    DependencyConfig {
                        id: "nodeNameScope".into(),
                        scope: DependencyScope::Production,
                        source: DependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "node".into(),
                    DependencyConfig {
                        id: "node".into(),
                        scope: DependencyScope::Development,
                        source: DependencySource::Explicit,
                        via: None
                    }
                ),
                (
                    "nodeNameOnly".into(),
                    DependencyConfig {
                        id: "nodeNameOnly".into(),
                        scope: DependencyScope::Peer,
                        source: DependencySource::Implicit,
                        via: Some("pkg-bar".to_string())
                    }
                )
            ])
        );
    }
}

mod type_constraints {
    use super::*;

    #[tokio::test]
    async fn app_can_use_unknown() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [unknown]");
        })
        .await;
    }

    #[tokio::test]
    async fn app_can_use_library() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [library]");
        })
        .await;
    }

    #[tokio::test]
    async fn app_can_use_tool() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [tool]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "InvalidTypeRelationship(\"app\", Application, \"app-other\", Application)"
    )]
    async fn app_cannot_use_app() {
        get_type_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("app/moon.yml"),
                "dependsOn: [app-other]",
            );
        })
        .await;
    }

    #[tokio::test]
    async fn library_can_use_unknown() {
        get_type_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("library/moon.yml"),
                "dependsOn: [unknown]",
            );
        })
        .await;
    }

    #[tokio::test]
    async fn library_can_use_library() {
        get_type_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("library/moon.yml"),
                "dependsOn: [library-other]",
            );
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "InvalidTypeRelationship(\"library\", Library, \"app\", Application)"
    )]
    async fn library_cannot_use_app() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [app]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidTypeRelationship(\"library\", Library, \"tool\", Tool)")]
    async fn library_cannot_use_tool() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [tool]");
        })
        .await;
    }

    #[tokio::test]
    async fn tool_can_use_unknown() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [unknown]");
        })
        .await;
    }

    #[tokio::test]
    async fn tool_can_use_library() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [library]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidTypeRelationship(\"tool\", Tool, \"app\", Application)")]
    async fn tool_cannot_use_app() {
        get_type_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [app]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidTypeRelationship(\"tool\", Tool, \"tool-other\", Tool)")]
    async fn tool_cannot_use_tool() {
        get_type_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("tool/moon.yml"),
                "dependsOn: [tool-other]",
            );
        })
        .await;
    }
}

mod tag_constraints {
    use super::*;

    #[tokio::test]
    async fn can_depon_tags_but_self_empty() {
        get_tag_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
            append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
            append_file(sandbox.path().join("c/moon.yml"), "tags: [druid]");
        })
        .await;
    }

    #[tokio::test]
    async fn ignores_unconfigured_relationships() {
        get_tag_constraints_graph(|sandbox| {
            append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
            append_file(sandbox.path().join("b/moon.yml"), "tags: [some]");
            append_file(sandbox.path().join("c/moon.yml"), "tags: [value]");
        })
        .await;
    }

    #[tokio::test]
    async fn matches_with_source_tag() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [warrior]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [warrior]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidTagRelationship(\"a\", \"warrior\", \"b\",")]
    async fn errors_for_no_source_tag_match() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [warrior]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
        })
        .await;
    }

    #[tokio::test]
    async fn matches_with_allowed_tag() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [warrior]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidTagRelationship(\"a\", \"warrior\", \"b\",")]
    async fn errors_for_no_allowed_tag_match() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [warrior]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
        })
        .await;
    }

    #[tokio::test]
    #[should_panic(expected = "Invalid tag relationship. Project a with tag mage cannot depend on")]
    async fn errors_for_depon_empty_tags() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [mage]",
            );
        })
        .await;
    }

    #[tokio::test]
    async fn matches_multiple_source_tags_to_a_single_allowed_tag() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b]\ntags: [warrior, mage]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [druid]");
        })
        .await;
    }

    #[tokio::test]
    async fn matches_single_source_tag_to_a_multiple_allowed_tags() {
        get_tag_constraints_graph(|sandbox| {
            append_file(
                sandbox.path().join("a/moon.yml"),
                "dependsOn: [b, c]\ntags: [mage]",
            );
            append_file(sandbox.path().join("b/moon.yml"), "tags: [druid, wizard]");
            append_file(
                sandbox.path().join("c/moon.yml"),
                "tags: [wizard, sorcerer, barbarian]",
            );
        })
        .await;
    }
}

mod query {
    use super::*;
    use moon_query::build_query;

    fn get_ids(projects: &[&Project]) -> Vec<String> {
        let mut ids = projects
            .iter()
            .map(|p| p.id.to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    #[tokio::test]
    async fn by_language() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("language!=[typescript,python]").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a", "d"]);
    }

    #[tokio::test]
    async fn by_project() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph.query(build_query("project~{b,d}").unwrap()).unwrap();

        assert_eq!(get_ids(&projects), vec!["b", "d"]);
    }

    #[tokio::test]
    async fn by_project_type() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("projectType!=[library]").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a", "c"]);
    }

    #[tokio::test]
    async fn by_project_source() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("projectSource~a").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a"]);
    }

    #[tokio::test]
    async fn by_tag() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("tag=[three,five]").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["b", "c"]);
    }

    #[tokio::test]
    async fn by_task() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("task=[test,build]").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a", "c", "d"]);
    }

    #[tokio::test]
    async fn by_task_platform() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("taskPlatform=[node]").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a", "b"]);

        let projects = graph
            .query(build_query("taskPlatform=system").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["c"]);
    }

    #[tokio::test]
    async fn by_task_type() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph.query(build_query("taskType=run").unwrap()).unwrap();

        assert_eq!(get_ids(&projects), vec!["a"]);
    }

    #[tokio::test]
    async fn with_and_conditions() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("task=build && taskPlatform=deno").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["d"]);
    }

    #[tokio::test]
    async fn with_or_conditions() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("language=javascript || language=typescript").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["a", "b"]);
    }

    #[tokio::test]
    async fn with_nested_conditions() {
        let (graph, _sandbox) = get_queries_graph().await;

        let projects = graph
            .query(build_query("projectType=library && (taskType=build || tag=three)").unwrap())
            .unwrap();

        assert_eq!(get_ids(&projects), vec!["b", "d"]);
    }
}
