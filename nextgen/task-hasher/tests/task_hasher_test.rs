use moon_config::{GlobPath, HasherConfig, HasherWalkStrategy, PortablePath};
use moon_task_hasher::TaskHasher;
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

        let mut hasher = TaskHasher::new(
            &project,
            project.get_task("testPatterns").unwrap(),
            &vcs,
            sandbox.path(),
            &hasher_config,
        );
        hasher.hash_inputs().await.unwrap();

        assert_eq!(
            hasher.hash().inputs.keys().collect::<Vec<_>>(),
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("files").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("files").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("dirs").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("dirs").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globStar").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globStar").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globNestedStar").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globNestedStar").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globGroup").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("globGroup").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
        }

        #[tokio::test]
        async fn includes_none() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (project_graph, vcs) = generate_project_graph(sandbox.path()).await;
            let project = project_graph.get("root").unwrap();
            let hasher_config = HasherConfig::default();

            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("none").unwrap(),
                &vcs,
                sandbox.path(),
                &hasher_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(
                hasher.hash().inputs.keys().collect::<Vec<_>>(),
                Vec::<&str>::new()
            );
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

            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("touched").unwrap(),
                &vcs,
                sandbox.path(),
                &hasher_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(
                hasher.hash().inputs.keys().collect::<Vec<_>>(),
                ["created.txt"]
            );
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutFile").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutFile").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutDir").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutDir").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutGlob").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inFileOutGlob").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutFile").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutFile").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutDir").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutDir").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
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
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutGlob").unwrap(),
                &vcs,
                sandbox.path(),
                &vcs_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);

            // Glob
            let mut hasher = TaskHasher::new(
                &project,
                project.get_task("inGlobOutGlob").unwrap(),
                &vcs,
                sandbox.path(),
                &glob_config,
            );
            hasher.hash_inputs().await.unwrap();

            assert_eq!(hasher.hash().inputs.keys().collect::<Vec<_>>(), expected);
        }
    }
}
