use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{
    DependencyScope, DependencySource, ProjectDependencyConfig, TaskDependencyCacheStrategy,
    TaskDependencyConfig, WorkspaceProjectGlobFormat, WorkspaceProjects, WorkspaceProjectsConfig,
};
use moon_project::{FileGroup, Project, ProjectAlias};
use moon_project_graph::*;
use moon_query::build_query;
use moon_task::{Target, TaskFileInput, TaskFileOutput, TaskGlobInput};
use moon_test_utils::{
    MoonSandbox, WorkspaceGraph, WorkspaceMockOptions, WorkspaceMocker, create_moon_sandbox,
};
use moon_workspace::WorkspaceGraphCacheState;
use rustc_hash::FxHashMap;
use starbase_sandbox::assert_snapshot;
use starbase_utils::{fs, json, string_vec};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

pub fn append_file<P: AsRef<Path>>(path: P, data: &str) {
    let mut file = OpenOptions::new().append(true).open(path.as_ref()).unwrap();

    writeln!(file, "\n\n{data}").unwrap();
}

fn map_ids(ids: Vec<Id>) -> Vec<String> {
    ids.into_iter().map(|id| id.to_string()).collect()
}

fn map_ids_from_target(targets: Vec<Target>) -> Vec<String> {
    targets
        .into_iter()
        .map(|target| target.get_task_id().unwrap().to_string())
        .collect()
}

fn get_ids_from_projects(projects: Vec<Arc<Project>>) -> Vec<String> {
    let mut ids = projects
        .iter()
        .map(|p| p.id.to_string())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

pub fn create_workspace_mocker(root: &Path) -> WorkspaceMocker {
    WorkspaceMocker::new(root)
        .load_default_configs()
        .with_default_projects()
        .with_all_toolchains()
        .with_inherited_tasks()
}

pub async fn build_graph(root: &Path, sync: bool) -> WorkspaceGraph {
    create_workspace_mocker(root)
        .mock_workspace_graph_with_options(WorkspaceMockOptions {
            sync,
            ..Default::default()
        })
        .await
}

pub async fn build_graph_from_fixture(fixture: &str) -> (MoonSandbox, WorkspaceGraph) {
    let sandbox = create_moon_sandbox(fixture);
    let graph = build_graph(sandbox.path(), fixture.contains("cycle")).await;

    (sandbox, graph)
}

pub async fn build_graph_from_fixture_for_builder(
    fixture: &str,
    async_graph: bool,
) -> (MoonSandbox, WorkspaceGraph) {
    let sandbox = create_moon_sandbox(fixture);
    let mut mock = create_workspace_mocker(sandbox.path());

    if async_graph {
        mock = mock.update_workspace_config(|config| {
            config.experiments.async_graph_building = true;
        });
    }

    let graph = mock
        .mock_workspace_graph_with_options(WorkspaceMockOptions::default())
        .await;

    (sandbox, graph)
}

mod project_graph {
    use super::*;

    fn dep(target: &str) -> TaskDependencyConfig {
        TaskDependencyConfig {
            cache_strategy: Some(TaskDependencyCacheStrategy::Ignored),
            ..TaskDependencyConfig::new(Target::parse(target).unwrap())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn gets_by_id() {
        let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

        assert!(graph.get_project("a").is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No project has been configured with the identifier or alias z")]
    async fn errors_unknown_id() {
        let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

        graph.get_project("z").unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn gets_by_path() {
        let (sandbox, graph) = build_graph_from_fixture("dependencies").await;

        assert_eq!(
            graph
                .get_project_from_path(Some(&sandbox.path().join("c/moon.yml")))
                .unwrap()
                .id,
            "c"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No project could be located starting from path z/moon.yml")]
    async fn errors_non_matching_path() {
        let (sandbox, graph) = build_graph_from_fixture("dependencies").await;

        graph
            .get_project_from_path(Some(&sandbox.path().join("z/moon.yml")))
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "A project already exists with the identifier id")]
    async fn errors_duplicate_ids() {
        build_graph_from_fixture("dupe-folder-conflict").await;
    }

    mod sources {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn globs() {
            let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn globs_with_root() {
            let sandbox = create_moon_sandbox("dependencies");
            let root = sandbox.path().join("dir");

            // Move files so that we can infer a compatible root project name
            fs::copy_dir_all(sandbox.path(), &root).unwrap();

            let graph = create_workspace_mocker(&root)
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Globs(string_vec!["*", "."]);
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c", "d", "dir"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn globs_with_config() {
            let sandbox = create_moon_sandbox("locate-configs");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Globs(string_vec!["*/moon.yml"]);
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "c"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn globs_with_config_and_root() {
            let sandbox = create_moon_sandbox("locate-configs");
            sandbox.create_file("moon.yml", "");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Globs(string_vec!["**/moon.yml"]);
                })
                .mock_workspace_graph()
                .await;

            let ids = get_ids_from_projects(graph.get_projects().unwrap());

            // Because the root project inherits the sandbox folder name,
            // and the sandbox name is randomly generated, we can't exact match
            assert_eq!(ids.len(), 3);
            assert!(ids.contains(&String::from("a")));
            assert!(ids.contains(&String::from("c")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn paths() {
            let sandbox = create_moon_sandbox("dependencies");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Sources(FxHashMap::from_iter([
                        (Id::raw("c"), "c".into()),
                        (Id::raw("b"), "b".into()),
                    ]));
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["b", "c"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn paths_and_globs() {
            let sandbox = create_moon_sandbox("dependencies");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Both(WorkspaceProjectsConfig {
                        globs: string_vec!["{a,c}"],
                        sources: FxHashMap::from_iter([
                            (Id::raw("b"), "b".into()),
                            (Id::raw("root"), ".".into()),
                        ]),
                        ..Default::default()
                    });
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c", "root"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_git_moon_folders() {
            let sandbox = create_moon_sandbox("dependencies");

            sandbox.create_file(".moon/workspace.yml", "projects: ['*']");
            sandbox.enable_git();

            let graph = build_graph(sandbox.path(), false).await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn filters_dot_folders() {
            let sandbox = create_moon_sandbox("dependencies");
            sandbox.create_file(".foo/moon.yml", "");

            let graph = build_graph(sandbox.path(), false).await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c", "d"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn filters_using_gitignore() {
            let sandbox = create_moon_sandbox("layer-constraints");

            sandbox.create_file(".gitignore", "*-other");
            sandbox.enable_git();

            let graph = build_graph(sandbox.path(), false).await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["app", "library", "tool", "unknown"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_different_id_casings() {
            let (_sandbox, graph) = build_graph_from_fixture("ids").await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                [
                    "Capital",
                    "PascalCase",
                    "With_nums-123",
                    "camelCase",
                    "kebab-case",
                    "snake_case"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_id_dir_name_format() {
            let sandbox = create_moon_sandbox("id-formats");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Both(WorkspaceProjectsConfig {
                        globs: string_vec!["**/moon.yml"],
                        glob_format: WorkspaceProjectGlobFormat::DirName,
                        ..Default::default()
                    });
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["five", "one", "three", "two"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_id_source_path_format() {
            let sandbox = create_moon_sandbox("id-formats");

            let graph = create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.projects = WorkspaceProjects::Both(WorkspaceProjectsConfig {
                        globs: string_vec!["**/moon.yml"],
                        glob_format: WorkspaceProjectGlobFormat::SourcePath,
                        ..Default::default()
                    });
                })
                .mock_workspace_graph()
                .await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["four/five", "one", "one/two", "one/two/three"]
            );
        }
    }

    mod cache {
        use super::*;

        const CACHE_PATH: &str = ".moon/cache/states/workspaceGraph.json";
        const STATE_PATH: &str = ".moon/cache/states/workspaceGraphStateV1.json";

        // Written by the `tc-tier1` test plugin when `extend_project_graph`
        // is called, allowing us to detect if/when it was invoked
        const MARKER_PATH: &str = ".moon/cache/tcExtendProjectGraph";

        fn load_state(sandbox: &MoonSandbox) -> WorkspaceGraphCacheState {
            json::read_file(sandbox.path().join(STATE_PATH)).unwrap()
        }

        async fn do_generate(root: &Path, async_graph: bool) -> WorkspaceGraph {
            let mut mock = create_workspace_mocker(root);

            if async_graph {
                mock = mock.update_workspace_config(|config| {
                    config.experiments.async_graph_building = true;
                });
            }

            mock.mock_workspace_graph_with_options(WorkspaceMockOptions {
                cache: root.join(".git").exists(),
                ..Default::default()
            })
            .await
        }

        async fn do_generate_with_plugins(root: &Path, async_graph: bool) -> WorkspaceGraph {
            let mut mock = WorkspaceMocker::new(root)
                .load_default_configs()
                .with_default_projects()
                .with_test_toolchains()
                .with_inherited_tasks();

            if async_graph {
                mock = mock.update_workspace_config(|config| {
                    config.experiments.async_graph_building = true;
                });
            }

            mock.mock_workspace_graph_with_options(WorkspaceMockOptions {
                cache: true,
                ..Default::default()
            })
            .await
        }

        async fn build_cached_graph(
            async_graph: bool,
            func: impl FnOnce(&MoonSandbox),
        ) -> (MoonSandbox, WorkspaceGraph) {
            let sandbox = create_moon_sandbox("dependencies");

            func(&sandbox);

            let graph = do_generate(sandbox.path(), async_graph).await;

            (sandbox, graph)
        }

        async fn test_invalidate(async_graph: bool, func: impl FnOnce(&MoonSandbox)) {
            let (sandbox, _graph) = build_cached_graph(async_graph, |sandbox| {
                sandbox.enable_git();
            })
            .await;

            let state1 = load_state(&sandbox);

            func(&sandbox);
            do_generate(sandbox.path(), async_graph).await;

            let state2 = load_state(&sandbox);

            assert_ne!(state1.last_hash, state2.last_hash);
        }

        async fn build_plugins_cached_graph(
            async_graph: bool,
            func: impl FnOnce(&MoonSandbox),
        ) -> MoonSandbox {
            let sandbox = create_moon_sandbox("dependencies");
            sandbox.enable_git();

            func(&sandbox);

            do_generate_with_plugins(sandbox.path(), async_graph).await;

            sandbox
        }

        async fn test_plugins_invalidate(
            async_graph: bool,
            setup: impl FnOnce(&MoonSandbox),
            mutate: impl FnOnce(&MoonSandbox),
        ) {
            let sandbox = build_plugins_cached_graph(async_graph, setup).await;

            let state1 = load_state(&sandbox);

            mutate(&sandbox);
            do_generate_with_plugins(sandbox.path(), async_graph).await;

            let state2 = load_state(&sandbox);

            assert_ne!(state1.last_hash, state2.last_hash);
        }

        // Generates the entire caching suite for the sync or async builder
        macro_rules! cache_tests {
            ($async_graph:expr) => {
                #[tokio::test(flavor = "multi_thread")]
                async fn doesnt_cache_if_no_vcs() {
                    let (sandbox, _graph) = build_cached_graph($async_graph, |_| {}).await;

                    assert!(!sandbox.path().join(CACHE_PATH).exists())
                }

                #[tokio::test(flavor = "multi_thread")]
                async fn caches_if_vcs() {
                    let (sandbox, _graph) = build_cached_graph($async_graph, |sandbox| {
                        sandbox.enable_git();
                    })
                    .await;

                    assert!(sandbox.path().join(CACHE_PATH).exists());
                }

                #[tokio::test(flavor = "multi_thread")]
                async fn loads_from_cache() {
                    let (sandbox, graph) = build_cached_graph($async_graph, |sandbox| {
                        sandbox.enable_git();
                    })
                    .await;
                    let cached_graph = do_generate(sandbox.path(), $async_graph).await;

                    assert_eq!(
                        graph.projects.get_node_keys(),
                        cached_graph.projects.get_node_keys()
                    );
                    assert_eq!(
                        graph.tasks.get_node_keys(),
                        cached_graph.tasks.get_node_keys()
                    );
                }

                #[tokio::test(flavor = "multi_thread")]
                async fn creates_states_and_manifests() {
                    let (sandbox, _graph) = build_cached_graph($async_graph, |sandbox| {
                        sandbox.enable_git();
                    })
                    .await;

                    let state = load_state(&sandbox);

                    assert!(!state.last_hash.as_str().is_empty());

                    assert!(
                        sandbox
                            .path()
                            .join(".moon/cache/hashes")
                            .join(format!("{}.json", state.last_hash))
                            .exists()
                    );
                }

                mod invalidation {
                    use super::*;

                    #[tokio::test(flavor = "multi_thread")]
                    async fn with_workspace_changes() {
                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file(".moon/workspace.yml", "# Changes");
                        })
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn with_toolchain_changes() {
                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file(".moon/toolchains.yml", "# Changes");
                        })
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn with_scoped_tasks_changes() {
                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file(".moon/tasks/node.yml", "# Changes");
                        })
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn with_project_config_changes() {
                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file("a/moon.yml", "# Changes");
                        })
                        .await;

                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file("b/moon.yml", "# Changes");
                        })
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn with_new_source_add() {
                        test_invalidate($async_graph, |sandbox| {
                            sandbox.create_file("z/moon.yml", "# Changes");
                        })
                        .await;
                    }
                }

                mod plugins {
                    use super::*;

                    #[tokio::test(flavor = "multi_thread")]
                    async fn skips_extend_project_graph_on_cache_hit() {
                        let sandbox = build_plugins_cached_graph($async_graph, |_| {}).await;
                        let marker = sandbox.path().join(MARKER_PATH);

                        // Called on the initial build
                        assert!(marker.exists());

                        fs::remove_file(&marker).unwrap();

                        // But not on a warm cache
                        do_generate_with_plugins(sandbox.path(), $async_graph).await;

                        assert!(!marker.exists());
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn calls_extend_project_graph_on_cache_miss() {
                        let sandbox = build_plugins_cached_graph($async_graph, |_| {}).await;
                        let marker = sandbox.path().join(MARKER_PATH);

                        fs::remove_file(&marker).unwrap();

                        // Invalidate by changing a project config
                        sandbox.create_file("a/moon.yml", "# Changes");

                        do_generate_with_plugins(sandbox.path(), $async_graph).await;

                        assert!(marker.exists());
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn resolves_plugin_aliases_on_cache_hit() {
                        let sandbox = build_plugins_cached_graph($async_graph, |sandbox| {
                            sandbox.create_file("a/tc.cfg", "a-alias");
                        })
                        .await;

                        // Warm run, uses the cached graph
                        let graph = do_generate_with_plugins(sandbox.path(), $async_graph).await;

                        assert_eq!(graph.get_project("a-alias").unwrap().id, Id::raw("a"));
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn invalidates_with_removed_source() {
                        // Prime the cache with a project that is removed before
                        // the next graph build. This mirrors reverting a newly
                        // created project from a workspace.
                        let sandbox = build_plugins_cached_graph($async_graph, |sandbox| {
                            sandbox.create_file("z/moon.yml", "# Changes");
                        })
                        .await;

                        let state1 = load_state(&sandbox);

                        fs::remove_dir_all(sandbox.path().join("z")).unwrap();

                        let graph = do_generate_with_plugins(sandbox.path(), $async_graph).await;
                        let state2 = load_state(&sandbox);

                        assert_ne!(state1.last_hash, state2.last_hash);
                        assert!(graph.get_project("z").is_err());
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn invalidates_with_new_manifest_file() {
                        test_plugins_invalidate(
                            $async_graph,
                            |_| {},
                            |sandbox| {
                                sandbox.create_file("a/tc.cfg", "a-alias");
                            },
                        )
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn invalidates_with_changed_manifest_file() {
                        test_plugins_invalidate(
                            $async_graph,
                            |sandbox| {
                                sandbox.create_file("a/tc.cfg", "a-alias");
                            },
                            |sandbox| {
                                sandbox.create_file("a/tc.cfg", "a-alias-changed");
                            },
                        )
                        .await;
                    }

                    #[tokio::test(flavor = "multi_thread")]
                    async fn invalidates_with_removed_manifest_file() {
                        test_plugins_invalidate(
                            $async_graph,
                            |sandbox| {
                                sandbox.create_file("a/tc.cfg", "a-alias");
                            },
                            |sandbox| {
                                fs::remove_file(sandbox.path().join("a/tc.cfg")).unwrap();
                            },
                        )
                        .await;
                    }
                }
            };
        }

        mod async_builder {
            use super::*;

            cache_tests!(true);
        }

        mod sync_builder {
            use super::*;

            cache_tests!(false);
        }

        mod interop {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn rebuilds_when_cache_written_by_other_builder() {
                let sandbox = create_moon_sandbox("dependencies");
                sandbox.enable_git();

                // Prime the cache with the sync builder
                do_generate(sandbox.path(), false).await;

                let state1 = load_state(&sandbox);

                // The async builder must rebuild with a different hash, and
                // not fail deserializing the other builder's cached shape
                do_generate(sandbox.path(), true).await;

                let state2 = load_state(&sandbox);

                assert_ne!(state1.last_hash, state2.last_hash);

                // And switching back must rebuild with a stable hash
                do_generate(sandbox.path(), false).await;

                let state3 = load_state(&sandbox);

                assert_eq!(state1.last_hash, state3.last_hash);
            }
        }
    }

    mod cycles {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn can_generate_with_cycles() {
            let (_sandbox, graph) = build_graph_from_fixture("cycle").await;

            assert_eq!(
                get_ids_from_projects(graph.get_projects().unwrap()),
                ["a", "b", "c"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("a").unwrap())
                ),
                ["b"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("b").unwrap())
                ),
                ["c"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("c").unwrap())
                ),
                string_vec![]
            );
        }

        async fn assert_cross_partition_cycle(async_graph: bool) {
            // a -> b (production), b -> a (development)
            let (_sandbox, graph) =
                build_graph_from_fixture_for_builder("dev-prod-loop", async_graph).await;

            let a = graph.get_project("a").unwrap();
            let b = graph.get_project("b").unwrap();

            assert_eq!(map_ids(graph.projects.dependencies_of(&a)), ["b"]);
            assert_eq!(map_ids(graph.projects.dependencies_of(&b)), ["a"]);
            assert_eq!(map_ids(graph.projects.dependents_of(&a)), ["b"]);
            assert_eq!(map_ids(graph.projects.dependents_of(&b)), ["a"]);

            // Deep traversals terminate, and include the starting project,
            // since it's reachable through the loop
            assert_eq!(map_ids(graph.projects.deep_dependencies_of(&a)), ["b", "a"]);

            // The union cycles, but each partition can still be sorted
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .partitioned_toposort(ScopePartition::Production)
                ),
                ["b", "a"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .partitioned_toposort(ScopePartition::Development)
                ),
                ["a", "b"]
            );
        }

        async fn assert_focus_across_partition_cycle(async_graph: bool) {
            let (_sandbox, graph) =
                build_graph_from_fixture_for_builder("dev-prod-loop", async_graph).await;

            let focused = graph.projects.focus_for(&Id::raw("a"), true).unwrap();

            let mut ids = map_ids(focused.get_node_keys());
            ids.sort();

            assert_eq!(ids, ["a", "b"]);
            assert_eq!(
                map_ids(focused.dependencies_of(&focused.get("a").unwrap())),
                ["b"]
            );
        }

        async fn assert_three_node_chain_cycle(async_graph: bool) {
            // a -> b (production), b -> c (production), c -> a (development)
            let (_sandbox, graph) =
                build_graph_from_fixture_for_builder("dev-prod-chain-loop", async_graph).await;

            let a = graph.get_project("a").unwrap();
            let c = graph.get_project("c").unwrap();

            assert_eq!(graph.projects.get_graph().edge_count(), 3);
            assert_eq!(map_ids(graph.projects.dependencies_of(&a)), ["b"]);
            assert_eq!(map_ids(graph.projects.dependencies_of(&c)), ["a"]);
            assert_eq!(
                map_ids(graph.projects.deep_dependencies_of(&c)),
                ["a", "b", "c"]
            );
        }

        async fn assert_cached_partition_cycle(async_graph: bool) {
            let sandbox = create_moon_sandbox("dev-prod-loop");
            sandbox.enable_git();

            // Prime the cache on the first pass, load from it on the second
            for _ in 0..2 {
                let mut mock = create_workspace_mocker(sandbox.path());

                if async_graph {
                    mock = mock.update_workspace_config(|config| {
                        config.experiments.async_graph_building = true;
                    });
                }

                let graph = mock
                    .mock_workspace_graph_with_options(WorkspaceMockOptions {
                        cache: true,
                        ..Default::default()
                    })
                    .await;

                let a = graph.get_project("a").unwrap();

                assert_eq!(graph.projects.get_graph().edge_count(), 2);
                assert_eq!(map_ids(graph.projects.dependencies_of(&a)), ["b"]);
                assert_eq!(map_ids(graph.projects.dependents_of(&a)), ["b"]);
            }
        }

        mod sync_builder {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn disconnects_same_scope_cycle() {
                // a -> b -> c -> a, all production
                let (_sandbox, graph) = build_graph_from_fixture_for_builder("cycle", false).await;

                assert_eq!(
                    map_ids(
                        graph
                            .projects
                            .dependencies_of(&graph.get_project("a").unwrap())
                    ),
                    ["b"]
                );
                assert_eq!(
                    map_ids(
                        graph
                            .projects
                            .dependencies_of(&graph.get_project("c").unwrap())
                    ),
                    string_vec![]
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn allows_cycles_that_cross_scope_partitions() {
                assert_cross_partition_cycle(false).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn allows_three_node_chain_cycles_across_partitions() {
                assert_three_node_chain_cycle(false).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn caches_partition_cycles() {
                assert_cached_partition_cycle(false).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn disconnects_same_partition_peer_loop() {
                // a -> b (peer), b -> a (production), same partition
                let (_sandbox, graph) =
                    build_graph_from_fixture_for_builder("peer-prod-loop", false).await;

                assert_eq!(graph.projects.get_graph().edge_count(), 1);
                assert_eq!(graph.projects.production_graph().edge_count(), 1);
                assert_eq!(graph.projects.development_graph().edge_count(), 0);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn disconnects_same_partition_build_dev_loop() {
                // a -> b (build), b -> a (development), same partition
                let (_sandbox, graph) =
                    build_graph_from_fixture_for_builder("build-dev-loop", false).await;

                assert_eq!(graph.projects.get_graph().edge_count(), 1);
                assert_eq!(graph.projects.production_graph().edge_count(), 0);
                assert_eq!(graph.projects.development_graph().edge_count(), 1);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn disconnects_self_dependencies() {
                let (_sandbox, graph) =
                    build_graph_from_fixture_for_builder("self-loop", false).await;

                assert_eq!(
                    map_ids(
                        graph
                            .projects
                            .dependencies_of(&graph.get_project("a").unwrap())
                    ),
                    string_vec![]
                );
                assert_eq!(graph.projects.get_graph().edge_count(), 0);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_focus_across_a_partition_cycle() {
                assert_focus_across_partition_cycle(false).await;
            }

            // No async builder variant, as its pooled build order isn't
            // deterministic, so node indexes may differ between runs
            #[tokio::test(flavor = "multi_thread")]
            async fn renders_a_partition_cycle_to_dot() {
                let (_sandbox, graph) =
                    build_graph_from_fixture_for_builder("dev-prod-loop", false).await;

                assert_snapshot!(graph.projects.to_dot());
            }
        }

        mod async_builder {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "project_graph::would_cycle")]
            async fn errors_for_same_scope_cycle() {
                build_graph_from_fixture_for_builder("cycle", true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "project_graph::would_cycle")]
            async fn errors_for_same_partition_peer_loop() {
                build_graph_from_fixture_for_builder("peer-prod-loop", true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "project_graph::would_cycle")]
            async fn errors_for_same_partition_build_dev_loop() {
                build_graph_from_fixture_for_builder("build-dev-loop", true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            #[should_panic(expected = "project_graph::would_cycle")]
            async fn errors_for_self_dependencies() {
                build_graph_from_fixture_for_builder("self-loop", true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn allows_cycles_that_cross_scope_partitions() {
                assert_cross_partition_cycle(true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn allows_three_node_chain_cycles_across_partitions() {
                assert_three_node_chain_cycle(true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn caches_partition_cycles() {
                assert_cached_partition_cycle(true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn can_focus_across_a_partition_cycle() {
                assert_focus_across_partition_cycle(true).await;
            }
        }
    }

    mod scope_partitions {
        use super::*;

        async fn assert_partitioned_graphs(async_graph: bool) {
            let (_sandbox, graph) =
                build_graph_from_fixture_for_builder("dependencies", async_graph).await;
            let projects = &graph.projects;

            // a -> b (development), b -> c (production),
            // d -> c (production), d -> b (build), d -> a (peer)
            assert_eq!(projects.get_graph().edge_count(), 5);
            assert_eq!(projects.production_graph().edge_count(), 3);
            assert_eq!(projects.development_graph().edge_count(), 2);

            let mut production_scopes = projects
                .production_graph()
                .edge_weights()
                .copied()
                .collect::<Vec<_>>();
            production_scopes.sort();

            assert_eq!(
                production_scopes,
                [
                    DependencyScope::Peer,
                    DependencyScope::Production,
                    DependencyScope::Production
                ]
            );

            let mut development_scopes = projects
                .development_graph()
                .edge_weights()
                .copied()
                .collect::<Vec<_>>();
            development_scopes.sort();

            assert_eq!(
                development_scopes,
                [DependencyScope::Build, DependencyScope::Development]
            );

            // All graphs share the same nodes and indexes
            assert_eq!(projects.get_graph().node_count(), 4);
            assert_eq!(projects.production_graph().node_count(), 4);
            assert_eq!(projects.development_graph().node_count(), 4);
        }

        async fn assert_partitioned_traversals(async_graph: bool) {
            // a -> b (development), b -> c (production),
            // d -> c (production), d -> b (build), d -> a (peer)
            let (_sandbox, graph) =
                build_graph_from_fixture_for_builder("dependencies", async_graph).await;
            let projects = &graph.projects;

            let a = graph.get_project("a").unwrap();
            let b = graph.get_project("b").unwrap();
            let d = graph.get_project("d").unwrap();

            // Direct dependencies
            let mut deps =
                map_ids(projects.partitioned_dependencies_of(&d, ScopePartition::Production));
            deps.sort();

            assert_eq!(deps, ["a", "c"]);
            assert_eq!(
                map_ids(projects.partitioned_dependencies_of(&d, ScopePartition::Development)),
                ["b"]
            );

            // Direct dependents
            let mut deps =
                map_ids(projects.partitioned_dependents_of(&b, ScopePartition::Development));
            deps.sort();

            assert_eq!(deps, ["a", "d"]);
            assert_eq!(
                map_ids(projects.partitioned_dependents_of(&b, ScopePartition::Production)),
                string_vec![]
            );

            // Deep traversals
            let mut deps =
                map_ids(projects.partitioned_deep_dependencies_of(&d, ScopePartition::Production));
            deps.sort();

            assert_eq!(deps, ["a", "c"]);
            assert_eq!(
                map_ids(projects.partitioned_deep_dependencies_of(&a, ScopePartition::Development)),
                ["b"]
            );

            let mut deps =
                map_ids(projects.partitioned_deep_dependents_of(&b, ScopePartition::Development));
            deps.sort();

            assert_eq!(deps, ["a", "d"]);

            // Topological ordering (dependencies first)
            let order = map_ids(projects.partitioned_toposort(ScopePartition::Production));
            let pos = |id: &str| order.iter().position(|order_id| order_id == id).unwrap();

            assert_eq!(order.len(), 4);
            assert!(pos("c") < pos("b")); // b -> c
            assert!(pos("c") < pos("d")); // d -> c
            assert!(pos("a") < pos("d")); // d -> a
        }

        mod sync_builder {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn routes_edges_into_partitioned_graphs() {
                assert_partitioned_graphs(false).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn traverses_within_a_partition() {
                assert_partitioned_traversals(false).await;
            }
        }

        mod async_builder {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn routes_edges_into_partitioned_graphs() {
                assert_partitioned_graphs(true).await;
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn traverses_within_a_partition() {
                assert_partitioned_traversals(true).await;
            }
        }
    }

    mod inheritance {
        use super::*;

        async fn build_inheritance_graph(fixture: &str) -> WorkspaceGraph {
            let sandbox = create_moon_sandbox(fixture);

            create_workspace_mocker(sandbox.path())
                .load_inherited_tasks_from(".moon")
                .mock_workspace_graph()
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_scoped_tasks() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("node").unwrap().task_targets.clone()),
                ["global", "global-javascript", "global-node", "node"]
            );

            assert_eq!(
                map_ids_from_target(
                    graph
                        .get_project("system-library")
                        .unwrap()
                        .task_targets
                        .clone()
                ),
                ["global", "global-system", "system-library"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_scoped_tasks_for_tier3_language() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(
                    graph
                        .get_project("node-library")
                        .unwrap()
                        .task_targets
                        .clone()
                ),
                [
                    "global",
                    "global-javascript",
                    "global-node",
                    "global-node-library",
                    "node-library"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_scoped_tasks_for_tier2_language() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("ruby-tool").unwrap().task_targets.clone()),
                ["global", "ruby-tool"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_scoped_tasks_for_custom_language() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(
                    graph
                        .get_project("kotlin-app")
                        .unwrap()
                        .task_targets
                        .clone()
                ),
                // kotlin is not a toolchain
                ["global", "kotlin-app"] // ["global", "global-kotlin", "global-system", "kotlin-app"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_js_tasks_for_bun_toolchain() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("bun").unwrap().task_targets.clone()),
                ["bun", "global", "global-javascript", "global-node"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_js_tasks_for_deno_toolchain() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("deno").unwrap().task_targets.clone()),
                ["deno", "global", "global-javascript", "global-node"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_js_tasks_for_node_toolchain() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("node").unwrap().task_targets.clone()),
                ["global", "global-javascript", "global-node", "node"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_ts_tasks_instead_of_js() {
            let graph = build_inheritance_graph("inheritance/scoped").await;

            assert_eq!(
                map_ids_from_target(
                    graph
                        .get_project("bun-with-ts")
                        .unwrap()
                        .task_targets
                        .clone()
                ),
                [
                    "bun",
                    "global",
                    "global-javascript",
                    "global-node",
                    "global-typescript"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_tagged_tasks() {
            let graph = build_inheritance_graph("inheritance/tagged").await;

            assert_eq!(
                map_ids_from_target(graph.get_project("mage").unwrap().task_targets.clone()),
                ["mage", "magic"]
            );

            assert_eq!(
                map_ids_from_target(graph.get_project("warrior").unwrap().task_targets.clone()),
                ["warrior", "weapons"]
            );

            assert_eq!(
                map_ids_from_target(graph.get_project("priest").unwrap().task_targets.clone()),
                ["magic", "priest", "weapons"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_file_groups() {
            let graph = build_inheritance_graph("inheritance/file-groups").await;
            let project = graph.get_project("project").unwrap();

            assert_eq!(
                project.file_groups.get("sources").unwrap(),
                &FileGroup::new_with_source(
                    "sources",
                    [
                        WorkspaceRelativePathBuf::from("project/sources/**/*"),
                        WorkspaceRelativePathBuf::from("project/src/**/*")
                    ]
                )
                .unwrap()
            );
            assert_eq!(
                project.file_groups.get("tests").unwrap(),
                &FileGroup::new_with_source(
                    "tests",
                    [WorkspaceRelativePathBuf::from("project/tests/**/*")]
                )
                .unwrap()
            );
            assert_eq!(
                project.file_groups.get("configs").unwrap(),
                &FileGroup::new_with_source(
                    "configs",
                    [WorkspaceRelativePathBuf::from("project/config.*")]
                )
                .unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_implicit_deps_inputs() {
            let graph = build_inheritance_graph("inheritance/implicits").await;
            let task = graph.get_task_from_project("project", "example").unwrap();

            assert_eq!(task.deps, [dep("project:other"), dep("base:local")]);

            assert_eq!(
                task.input_files,
                FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("project/local.txt"),
                    TaskFileInput::default()
                )])
            );

            assert_eq!(
                task.input_globs,
                FxHashMap::from_iter([
                    (
                        WorkspaceRelativePathBuf::from(
                            ".moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                        ),
                        TaskGlobInput::default()
                    ),
                    (
                        WorkspaceRelativePathBuf::from("project/global.*"),
                        TaskGlobInput::default()
                    )
                ])
            );
        }
    }

    mod expansion {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn expands_project() {
            let (_sandbox, graph) = build_graph_from_fixture("expansion").await;
            let project = graph.get_project("project").unwrap();

            assert_eq!(
                project.dependencies,
                vec![ProjectDependencyConfig {
                    id: Id::raw("base"),
                    scope: DependencyScope::Development,
                    source: DependencySource::Explicit,
                    ..Default::default()
                }]
            );

            assert!(
                graph
                    .get_task_from_project("project", "build")
                    .unwrap()
                    .deps
                    .is_empty()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn expands_tasks() {
            let (_sandbox, graph) = build_graph_from_fixture("expansion").await;
            let task = graph.get_task_from_project("tasks", "build").unwrap();

            assert_eq!(
                task.args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>(),
                string_vec![
                    "a",
                    if cfg!(windows) {
                        "..\\other.yaml"
                    } else {
                        "../other.yaml"
                    },
                    "b"
                ]
            );

            assert_eq!(
                task.input_files,
                FxHashMap::from_iter([
                    (
                        WorkspaceRelativePathBuf::from("tasks/config.json"),
                        TaskFileInput::default()
                    ),
                    (
                        WorkspaceRelativePathBuf::from("other.yaml"),
                        TaskFileInput::default()
                    ),
                ])
            );

            assert_eq!(
                task.input_globs,
                FxHashMap::from_iter([
                    (
                        WorkspaceRelativePathBuf::from(
                            ".moon/*.{yml,yaml,jsonc,json,pkl,hcl,toml}"
                        ),
                        TaskGlobInput::default()
                    ),
                    (
                        WorkspaceRelativePathBuf::from("tasks/file.*"),
                        TaskGlobInput::default()
                    ),
                ])
            );

            assert_eq!(
                task.output_files,
                FxHashMap::from_iter([(
                    WorkspaceRelativePathBuf::from("tasks/build"),
                    TaskFileOutput::default()
                )])
            );

            assert_eq!(task.deps, [dep("project:build")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn expands_tag_deps_in_task() {
            let (_sandbox, graph) = build_graph_from_fixture("expansion").await;
            let task = graph.get_task_from_project("tasks", "test-tags").unwrap();

            assert_eq!(task.deps, [dep("tag-one:test"), dep("tag-three:test")]);
        }
    }

    mod dependencies {
        use super::*;

        fn dep_with_strategy(
            target: &str,
            strategy: TaskDependencyCacheStrategy,
        ) -> TaskDependencyConfig {
            TaskDependencyConfig {
                cache_strategy: Some(strategy),
                ..TaskDependencyConfig::new(Target::parse(target).unwrap())
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn defaults_cache_strategy_from_dep_outputs() {
            let (_sandbox, graph) = build_graph_from_fixture("dep-cache-strategy").await;
            let task = graph.get_task_from_project("consumer", "check").unwrap();

            // A dependency that declares `outputs` defaults to `hash`, while a
            // dependency without outputs defaults to `ignored`.
            assert_eq!(
                task.deps,
                [
                    dep_with_strategy("producer:build", TaskDependencyCacheStrategy::Hash),
                    dep_with_strategy("producer:lint", TaskDependencyCacheStrategy::Ignored),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn lists_ids_of_dependencies() {
            let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("a").unwrap())
                ),
                ["b"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("b").unwrap())
                ),
                ["c"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("c").unwrap())
                ),
                string_vec![]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("d").unwrap())
                ),
                ["b", "c", "a"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn lists_ids_of_dependents() {
            let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependents_of(&graph.get_project("a").unwrap())
                ),
                ["d"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependents_of(&graph.get_project("b").unwrap())
                ),
                ["d", "a"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependents_of(&graph.get_project("c").unwrap())
                ),
                ["d", "b"]
            );
            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependents_of(&graph.get_project("d").unwrap())
                ),
                string_vec![]
            );
        }

        mod isolation {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn no_depends_on() {
                let sandbox = create_moon_sandbox("dependency-types");
                let mock = create_workspace_mocker(sandbox.path());

                let graph = mock.mock_workspace_graph_for(&["no-depends-on"]).await;

                assert_eq!(map_ids(graph.projects.get_node_keys()), ["no-depends-on"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn some_depends_on() {
                let sandbox = create_moon_sandbox("dependency-types");
                let mock = create_workspace_mocker(sandbox.path());

                let graph = mock.mock_workspace_graph_for(&["some-depends-on"]).await;
                let project = graph.get_project("some-depends-on").unwrap();
                let mut direct_deps = map_ids(graph.projects.dependencies_of(&project));
                direct_deps.sort();

                assert_eq!(
                    map_ids(graph.projects.get_node_keys()),
                    ["some-depends-on", "a", "c"]
                );
                assert_eq!(direct_deps, ["a", "c"]);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn from_task_deps() {
                let sandbox = create_moon_sandbox("dependency-types");
                let mock = create_workspace_mocker(sandbox.path());

                let graph = mock.mock_workspace_graph_for(&["from-task-deps"]).await;
                let project = graph.get_project("from-task-deps").unwrap();
                let build = graph
                    .get_task_from_project("from-task-deps", "build")
                    .unwrap();
                let check = graph
                    .get_task_from_project("from-task-deps", "check")
                    .unwrap();
                let mut direct_deps = map_ids(graph.projects.dependencies_of(&project));
                direct_deps.sort();

                assert_eq!(
                    map_ids(graph.projects.get_node_keys()),
                    ["from-task-deps", "b", "c"]
                );
                assert_eq!(direct_deps, ["b", "c"]);
                assert_eq!(
                    graph.tasks.dependencies_of(&build),
                    vec![Target::parse("b:build").unwrap()]
                );
                assert_eq!(
                    graph.tasks.dependencies_of(&check),
                    vec![Target::parse("c:check").unwrap()]
                );

                let deps = &graph.get_project("from-task-deps").unwrap().dependencies;

                assert_eq!(deps[0].scope, DependencyScope::Build);
                assert_eq!(deps[1].scope, DependencyScope::Build);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn from_root_task_deps() {
                let sandbox = create_moon_sandbox("dependency-types");
                let mock = create_workspace_mocker(sandbox.path());

                let graph = mock
                    .mock_workspace_graph_for(&["from-root-task-deps"])
                    .await;
                let build = graph
                    .get_task_from_project("from-root-task-deps", "build")
                    .unwrap();

                assert_eq!(
                    map_ids(graph.projects.get_node_keys()),
                    ["from-root-task-deps", "root"]
                );
                assert_eq!(
                    graph.tasks.dependencies_of(&build),
                    vec![Target::parse("root:noop").unwrap()]
                );

                let deps = &graph
                    .get_project("from-root-task-deps")
                    .unwrap()
                    .dependencies;

                assert_eq!(deps[0].scope, DependencyScope::Root);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn self_task_deps() {
                let sandbox = create_moon_sandbox("dependency-types");
                let mock = create_workspace_mocker(sandbox.path());

                let graph = mock.mock_workspace_graph_for(&["self-task-deps"]).await;

                assert_eq!(map_ids(graph.projects.get_node_keys()), ["self-task-deps"]);
            }
        }
    }

    mod aliases {
        use super::*;

        async fn build_aliases_graph() -> (MoonSandbox, WorkspaceGraph) {
            build_aliases_graph_for_fixture("aliases").await
        }

        async fn build_aliases_graph_for_fixture(fixture: &str) -> (MoonSandbox, WorkspaceGraph) {
            let (sandbox, mocker) = build_aliases_graph_with_mocker(fixture).await;

            (sandbox, mocker.mock_workspace_graph().await)
        }

        async fn build_aliases_graph_with_mocker(fixture: &str) -> (MoonSandbox, WorkspaceMocker) {
            let sandbox = create_moon_sandbox(fixture);

            let mocker = create_workspace_mocker(sandbox.path())
                .with_default_projects()
                .with_all_toolchains();

            (sandbox, mocker)
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_aliases() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_snapshot!(graph.projects.to_dot());

            assert_eq!(
                graph.projects.aliases(),
                FxHashMap::from_iter([
                    ("one", &Id::raw("alias-one")),
                    ("two", &Id::raw("alias-two")),
                    ("three", &Id::raw("alias-three")),
                    ("rust_toolchain", &Id::raw("multiple")),
                    ("js-toolchain", &Id::raw("multiple")),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_multi_aliases_from_each_toolchain() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_eq!(
                graph.get_project("multiple").unwrap().aliases,
                [
                    ProjectAlias {
                        alias: "rust_toolchain".into(),
                        plugin: Id::raw("rust"),
                    },
                    ProjectAlias {
                        alias: "js-toolchain".into(),
                        plugin: Id::raw("javascript"),
                    },
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_disable_aliases_per_toolchain() {
            let (_sandbox, mut mocker) = build_aliases_graph_with_mocker("aliases").await;

            mocker = mocker.update_toolchains_config(|cfg| {
                if let Some(inner) = cfg.plugins.get_mut("rust") {
                    inner.inherit_aliases = false;
                }
            });

            let graph = mocker.mock_workspace_graph().await;

            assert_eq!(
                graph.get_project("multiple").unwrap().aliases,
                [ProjectAlias {
                    alias: "js-toolchain".into(),
                    plugin: Id::raw("javascript"),
                },]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_alias_if_same_as_id() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_eq!(
                graph.get_project("alias-same-id").unwrap().aliases,
                [ProjectAlias {
                    alias: "alias-same-id".into(),
                    plugin: Id::raw("javascript"),
                }]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_alias_if_a_project_has_the_id() {
            let (_sandbox, graph) = build_aliases_graph_for_fixture("aliases-conflict-ids").await;

            assert!(graph.get_project("one").unwrap().aliases.is_empty());
            assert!(graph.get_project("two").unwrap().aliases.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_get_projects_by_alias() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert!(graph.get_project("one").is_ok());
            assert!(graph.get_project("two").is_ok());
            assert!(graph.get_project("three").is_ok());

            assert_eq!(
                graph.get_project("one").unwrap(),
                graph.get_project("alias-one").unwrap()
            );
            assert_eq!(
                graph.get_project("two").unwrap(),
                graph.get_project("alias-two").unwrap()
            );
            assert_eq!(
                graph.get_project("three").unwrap(),
                graph.get_project("alias-three").unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_depends_on_by_alias() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("explicit").unwrap())
                ),
                ["alias-two", "alias-one"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("explicit-and-implicit").unwrap())
                ),
                ["alias-three", "alias-two"]
            );

            assert_eq!(
                map_ids(
                    graph
                        .projects
                        .dependencies_of(&graph.get_project("implicit").unwrap())
                ),
                ["alias-three", "alias-one"]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_or_flattens_dupes() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_eq!(
                graph.get_project("dupes-depends-on").unwrap().dependencies,
                vec![ProjectDependencyConfig {
                    id: Id::raw("alias-two"),
                    scope: DependencyScope::Build,
                    source: DependencySource::Explicit,
                    ..ProjectDependencyConfig::default()
                }]
            );

            assert_eq!(
                graph
                    .get_task_from_project("dupes-task-deps", "no-dupes")
                    .unwrap()
                    .deps,
                [dep("alias-one:global")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_use_aliases_as_task_deps() {
            let (_sandbox, graph) = build_aliases_graph().await;

            assert_eq!(
                graph
                    .get_task_from_project("tasks", "with-aliases")
                    .unwrap()
                    .deps,
                [
                    dep("alias-one:global"),
                    dep("alias-three:global"),
                    dep("implicit:global"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_duplicate_aliases_if_ids_match() {
            let sandbox = create_moon_sandbox("aliases-conflict");
            let mock = create_workspace_mocker(sandbox.path());
            let context = mock.mock_workspace_builder_context();

            let graph = mock
                .mock_workspace_graph_with_options(WorkspaceMockOptions {
                    context: Some(context),
                    ..Default::default()
                })
                .await;

            assert!(graph.get_project("one").is_ok());
            assert!(graph.get_project("two").is_ok());
        }
    }

    mod layer_constraints {
        use super::*;

        async fn build_layer_constraints_graph(func: impl FnOnce(&MoonSandbox)) -> WorkspaceGraph {
            let sandbox = create_moon_sandbox("layer-constraints");

            func(&sandbox);

            create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.constraints.enforce_layer_relationships = true;
                })
                .mock_workspace_graph()
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn app_can_use_unknown() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [unknown]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn app_can_use_library() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [library]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn app_can_use_tool() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("app/moon.yml"), "dependsOn: [tool]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Layering violation: Project app with layer application")]
        async fn app_cannot_use_app() {
            build_layer_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("app/moon.yml"),
                    "dependsOn: [app-other]",
                );
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn library_can_use_unknown() {
            build_layer_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("library/moon.yml"),
                    "dependsOn: [unknown]",
                );
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn library_can_use_library() {
            build_layer_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("library/moon.yml"),
                    "dependsOn: [library-other]",
                );
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Layering violation: Project library with layer library")]
        async fn library_cannot_use_app() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [app]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Layering violation: Project library with layer library")]
        async fn library_cannot_use_tool() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("library/moon.yml"), "dependsOn: [tool]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn tool_can_use_unknown() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [unknown]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn tool_can_use_library() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [library]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Layering violation: Project tool with layer tool")]
        async fn tool_cannot_use_app() {
            build_layer_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("tool/moon.yml"), "dependsOn: [app]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn tool_can_use_tool() {
            build_layer_constraints_graph(|sandbox| {
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

        async fn build_tag_constraints_graph(func: impl FnOnce(&MoonSandbox)) -> WorkspaceGraph {
            let sandbox = create_moon_sandbox("tag-constraints");

            func(&sandbox);

            create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.constraints.tag_relationships.insert(
                        Id::raw("warrior"),
                        vec![Id::raw("barbarian"), Id::raw("paladin"), Id::raw("druid")],
                    );

                    config.constraints.tag_relationships.insert(
                        Id::raw("mage"),
                        vec![Id::raw("wizard"), Id::raw("sorcerer"), Id::raw("druid")],
                    );
                })
                .mock_workspace_graph()
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_depon_tags_but_self_empty() {
            build_tag_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
                append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
                append_file(sandbox.path().join("c/moon.yml"), "tags: [druid]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_unconfigured_relationships() {
            build_tag_constraints_graph(|sandbox| {
                append_file(sandbox.path().join("a/moon.yml"), "dependsOn: [b, c]");
                append_file(sandbox.path().join("b/moon.yml"), "tags: [some]");
                append_file(sandbox.path().join("c/moon.yml"), "tags: [value]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn matches_with_source_tag() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [warrior]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid tag relationship: Project a with tag #warrior")]
        async fn errors_for_no_source_tag_match() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn matches_with_allowed_tag() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [barbarian]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid tag relationship: Project a with tag #warrior")]
        async fn errors_for_no_allowed_tag_match() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [other]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid tag relationship: Project a with tag #mage")]
        async fn errors_for_depon_empty_tags() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [mage]",
                );
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn matches_multiple_source_tags_to_a_single_allowed_tag() {
            build_tag_constraints_graph(|sandbox| {
                append_file(
                    sandbox.path().join("a/moon.yml"),
                    "dependsOn: [b]\ntags: [warrior, mage]",
                );
                append_file(sandbox.path().join("b/moon.yml"), "tags: [druid]");
            })
            .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn matches_single_source_tag_to_a_multiple_allowed_tags() {
            build_tag_constraints_graph(|sandbox| {
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

        #[tokio::test(flavor = "multi_thread")]
        async fn by_language() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("language!=[typescript,python]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "d"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_project() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("project~{b,d}").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "d"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_project_type() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("projectLayer!=[library]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "c"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_project_source() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("projectSource~a").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_tag() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("projectTag=[three,five]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "c"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_task() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("task=[test,build]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "c", "d"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_task_toolchain() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("taskToolchain=[node]").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);

            let projects = graph
                .query_projects(build_query("taskToolchain=python").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["c"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn by_task_type() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("taskType=run").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_and_conditions() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("task=build && taskToolchain=node").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_or_conditions() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(build_query("language=javascript || language=typescript").unwrap())
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["a", "b"]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn with_nested_conditions() {
            let (_sandbox, graph) = build_graph_from_fixture("query").await;

            let projects = graph
                .query_projects(
                    build_query("projectLayer=library && (taskType=build || projectTag=three)")
                        .unwrap(),
                )
                .unwrap();

            assert_eq!(get_ids_from_projects(projects), vec!["b", "d"]);
        }
    }

    mod to_dot {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn renders_full() {
            let (_sandbox, graph) = build_graph_from_fixture("dependencies").await;

            assert_snapshot!(graph.projects.to_dot());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn renders_partial() {
            let sandbox = create_moon_sandbox("dependencies");
            let mock = create_workspace_mocker(sandbox.path());

            let graph = mock.mock_workspace_graph_for(&["b"]).await;

            assert_snapshot!(graph.projects.to_dot());
        }
    }

    mod custom_id {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn can_load_by_new_id() {
            let sandbox = create_moon_sandbox("custom-id");
            let graph = build_graph(sandbox.path(), false).await;

            assert_eq!(graph.get_project("foo").unwrap().id, "foo");
            assert_eq!(graph.get_project("bar-renamed").unwrap().id, "bar-renamed");
            assert_eq!(graph.get_project("baz-renamed").unwrap().id, "baz-renamed");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn tasks_can_depend_on_new_id() {
            let sandbox = create_moon_sandbox("custom-id");
            let graph = build_graph(sandbox.path(), false).await;
            let task = graph.get_task_from_project("foo", "noop").unwrap();

            assert_eq!(
                task.deps,
                [dep("bar-renamed:noop"), dep("baz-renamed:noop")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_duplicate_folder_names_if_renamed() {
            let (_sandbox, graph) = build_graph_from_fixture("dupe-folder-ids").await;

            assert!(graph.get_project("one").is_ok());
            assert!(graph.get_project("two").is_ok());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "A project already exists with the identifier foo")]
        async fn errors_duplicate_ids_from_rename() {
            build_graph_from_fixture("custom-id-conflict").await;
        }
    }

    mod default_id {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Invalid default project")]
        async fn errors_if_default_id_doesnt_exist() {
            let sandbox = create_moon_sandbox("dependencies");

            create_workspace_mocker(sandbox.path())
                .update_workspace_config(|config| {
                    config.default_project = Some(Id::raw("z"));
                })
                .mock_workspace_graph()
                .await;
        }
    }
}
