use moon_config::{GlobPath, HasherConfig, HasherWalkStrategy, PortablePath};
use moon_project::Project;
use moon_task_hasher::{TaskHash, TaskHasher};
use moon_test_utils2::{ProjectGraph, ProjectGraphContainer};
use moon_vcs::BoxedVcs;
use starbase_sandbox::create_sandbox;
use std::fs;
use std::path::Path;

fn create_out_files(project_root: &Path) {
    let out_dir = project_root.join("out");

    fs::create_dir_all(&out_dir).unwrap();

    for i in 1..=5 {
        fs::write(out_dir.join(i.to_string()), i.to_string()).unwrap();
    }
}

fn create_hasher_configs() -> (HasherConfig, HasherConfig) {
    (
        HasherConfig {
            walk_strategy: HasherWalkStrategy::Vcs,
            ..HasherConfig::default()
        },
        HasherConfig {
            walk_strategy: HasherWalkStrategy::Glob,
            ..HasherConfig::default()
        },
    )
}

async fn generate_project_graph(workspace_root: &Path) -> (ProjectGraph, BoxedVcs) {
    let mut graph_builder = ProjectGraphContainer::with_vcs(workspace_root);
    let context = graph_builder.create_context();

    create_out_files(workspace_root);

    let graph = graph_builder.build_graph(context).await;
    let vcs = graph_builder.vcs.take().unwrap();

    (graph, vcs)
}

async fn generate_hash<'a>(
    project: &'a Project,
    task_name: &'a str,
    vcs: &'a BoxedVcs,
    workspace_root: &'a Path,
    hasher_config: &'a HasherConfig,
) -> TaskHash<'a> {
    let mut hasher = TaskHasher::new(
        project,
        project.get_task(task_name).unwrap(),
        vcs,
        workspace_root,
        hasher_config,
    );
    hasher.hash_inputs().await.unwrap();
    hasher.hash()
}

mod task_hasher {
    use super::*;

    #[tokio::test]
    async fn filters_out_files_matching_ignore_pattern() {
        let sandbox = create_sandbox("ignore-patterns");
        sandbox.enable_git();

        let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
        let project = project_graph.get("root").unwrap();

        let hasher_config = HasherConfig {
            ignore_patterns: vec![GlobPath::from_str("**/out/**").unwrap()],
            ..HasherConfig::default()
        };

        let result = generate_hash(
            &project,
            "testPatterns",
            &vcs,
            sandbox.path(),
            &hasher_config,
        )
        .await;

        assert_eq!(
            result.inputs.keys().collect::<Vec<_>>(),
            [".gitignore", "package.json"]
        );
    }

    mod input_aggregation {
        use super::*;

        #[tokio::test]
        async fn includes_files() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = ["2.txt", "dir/abc.txt"];

            // VCS
            let result = generate_hash(&project, "files", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(&project, "files", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_dirs() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = ["dir/abc.txt", "dir/az.txt", "dir/xyz.txt"];

            // VCS
            let result = generate_hash(&project, "dirs", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(&project, "dirs", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_globs_star() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = ["1.txt", "2.txt", "3.txt"];

            // VCS
            let result =
                generate_hash(&project, "globStar", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "globStar", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_globs_nested_star() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [
                "1.txt",
                "2.txt",
                "3.txt",
                "dir/abc.txt",
                "dir/az.txt",
                "dir/xyz.txt",
            ];

            // VCS
            let result = generate_hash(
                &project,
                "globNestedStar",
                &vcs,
                sandbox.path(),
                &vcs_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(
                &project,
                "globNestedStar",
                &vcs,
                sandbox.path(),
                &glob_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_globs_groups() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = ["dir/az.txt", "dir/xyz.txt"];

            // VCS
            let result =
                generate_hash(&project, "globGroup", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "globGroup", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_none() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let project = project_graph.get("root").unwrap();
            let hasher_config = HasherConfig::default();

            let result =
                generate_hash(&project, "none", &vcs, sandbox.path(), &hasher_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), Vec::<&str>::new());
        }

        #[tokio::test]
        async fn includes_local_touched_files() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();
            sandbox.create_file("created.txt", "");
            sandbox.create_file("filtered.txt", "");
            sandbox.run_git(|cmd| {
                cmd.args(["add", "created.txt", "filtered.txt"]);
            });

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let project = project_graph.get("root").unwrap();
            let hasher_config = HasherConfig::default();

            let result =
                generate_hash(&project, "touched", &vcs, sandbox.path(), &hasher_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), ["created.txt"]);
        }

        #[tokio::test]
        async fn includes_env_file() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();
            sandbox.create_file(".env", "");

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [".env"];

            // VCS
            let result =
                generate_hash(&project, "envFile", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "envFile", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_custom_env_files() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();
            sandbox.create_file(".env.prod", "");
            sandbox.create_file(".env.local", "");

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [".env.local", ".env.prod"];

            // VCS
            let result =
                generate_hash(&project, "envFileList", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "envFileList", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }
    }

    mod output_filtering {
        use super::*;

        #[tokio::test]
        async fn input_file_output_file() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "out/1",
                "out/3",
            ];

            // VCS
            let result =
                generate_hash(&project, "inFileOutFile", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(
                &project,
                "inFileOutFile",
                &vcs,
                sandbox.path(),
                &glob_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn input_file_output_dir() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [".moon/toolchain.yml", ".moon/workspace.yml"];

            // VCS
            let result =
                generate_hash(&project, "inFileOutDir", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "inFileOutDir", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn input_file_output_glob() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [".moon/toolchain.yml", ".moon/workspace.yml"];

            // VCS
            let result =
                generate_hash(&project, "inFileOutGlob", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(
                &project,
                "inFileOutGlob",
                &vcs,
                sandbox.path(),
                &glob_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn input_glob_output_file() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [
                ".gitignore",
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "out/1",
                "out/3",
                "out/5",
                "package.json",
            ];

            // VCS
            let result =
                generate_hash(&project, "inGlobOutFile", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(
                &project,
                "inGlobOutFile",
                &vcs,
                sandbox.path(),
                &glob_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn input_glob_output_dir() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [
                ".gitignore",
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "package.json",
            ];

            // VCS
            let result =
                generate_hash(&project, "inGlobOutDir", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result =
                generate_hash(&project, "inGlobOutDir", &vcs, sandbox.path(), &glob_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn input_glob_output_glob() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = project_graph.get("root").unwrap();

            let expected = [
                ".gitignore",
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "package.json",
            ];

            // VCS
            let result =
                generate_hash(&project, "inGlobOutGlob", &vcs, sandbox.path(), &vcs_config).await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let result = generate_hash(
                &project,
                "inGlobOutGlob",
                &vcs,
                sandbox.path(),
                &glob_config,
            )
            .await;

            assert_eq!(result.inputs.keys().collect::<Vec<_>>(), expected);
        }
    }
}
