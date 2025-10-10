use moon_app_context::AppContext;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{GlobPath, HasherConfig, HasherWalkStrategy, PortablePath};
use moon_project::Project;
use moon_task::Task;
use moon_task_hasher::{TaskHash, TaskHasher};
use moon_test_utils2::{WorkspaceGraph, WorkspaceMocker};
use starbase_sandbox::create_sandbox;
use std::collections::BTreeMap;
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

async fn mock_workspace(workspace_root: &Path) -> (WorkspaceGraph, AppContext) {
    create_out_files(workspace_root);

    let mock = WorkspaceMocker::new(workspace_root)
        .load_default_configs()
        .with_default_projects()
        .with_default_toolchains()
        .with_inherited_tasks()
        .with_global_envs();

    (mock.mock_workspace_graph().await, mock.mock_app_context())
}

async fn generate_hash<'a>(
    project: &'a Project,
    task: &'a Task,
    wg: &'a WorkspaceGraph,
    app: &'a AppContext,
    config: &'a HasherConfig,
) -> TaskHash<'a> {
    let mut hasher = TaskHasher::new(app, &wg.projects, project, task, config);
    hasher.hash_inputs().await.unwrap();
    hasher.hash()
}

fn get_input_files(
    mut inputs: BTreeMap<WorkspaceRelativePathBuf, String>,
) -> Vec<WorkspaceRelativePathBuf> {
    inputs.remove(&WorkspaceRelativePathBuf::from(".moon/cache/CACHEDIR.TAG"));
    inputs.into_keys().collect::<Vec<_>>()
}

mod task_hasher {
    use super::*;

    #[tokio::test]
    async fn filters_out_files_matching_ignore_pattern() {
        let sandbox = create_sandbox("ignore-patterns");
        sandbox.enable_git();

        let (wg, app) = mock_workspace(sandbox.path()).await;
        let project = wg.get_project("root").unwrap();
        let task = wg.get_task_from_project("root", "testPatterns").unwrap();

        let hasher_config = HasherConfig {
            ignore_patterns: vec![GlobPath::parse("**/out/**").unwrap()],
            ..HasherConfig::default()
        };

        let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

        assert_eq!(
            get_input_files(result.inputs),
            [".gitignore", "package.json"]
        );
    }

    mod input_aggregation {
        use super::*;

        #[tokio::test]
        async fn includes_files() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "files").unwrap();

            let expected = ["2.txt", "dir/abc.txt"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_dirs() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "dirs").unwrap();

            let expected = ["dir/abc.txt", "dir/az.txt", "dir/xyz.txt"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_globs_star() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "globStar").unwrap();

            let expected = ["1.txt", "2.txt", "3.txt"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_globs_nested_star() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "globNestedStar").unwrap();

            let expected = [
                "1.txt",
                "2.txt",
                "3.txt",
                "dir/abc.txt",
                "dir/az.txt",
                "dir/xyz.txt",
            ];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_globs_groups() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "globGroup").unwrap();

            let expected = ["dir/az.txt", "dir/xyz.txt"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn excludes_glob_negations() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "globNegated").unwrap();

            let expected = ["2.txt", "dir/abc.txt", "dir/xyz.txt"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_none() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "none").unwrap();

            let hasher_config = HasherConfig::default();

            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(get_input_files(result.inputs), Vec::<&str>::new());
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

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "touched").unwrap();

            let hasher_config = HasherConfig::default();

            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(get_input_files(result.inputs), ["created.txt"]);
        }

        #[tokio::test]
        async fn includes_env_file() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();
            sandbox.create_file(".env", "");

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "envFile").unwrap();

            let expected = [".env"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn includes_custom_env_files() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();
            sandbox.create_file(".env.prod", "");
            sandbox.create_file(".env.local", "");

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "envFileList").unwrap();

            let expected = [".env.local", ".env.prod"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn can_include_moon_project_config() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "moonConfig").unwrap();

            let hasher_config = HasherConfig::default();
            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(get_input_files(result.inputs), ["moon.yml"]);
        }

        #[tokio::test]
        #[should_panic(expected = "task_hasher::missing_input_file")]
        async fn errors_if_optional_false_and_file_missing() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, _) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "filesRequired").unwrap();

            generate_hash(&project, &task, &wg, &app, &vcs_config).await;
        }

        #[tokio::test]
        async fn doesnt_error_if_optional_true_and_file_missing() {
            let sandbox = create_sandbox("inputs");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, _) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "filesOptional").unwrap();

            let _ = generate_hash(&project, &task, &wg, &app, &vcs_config).await;
        }

        #[tokio::test]
        async fn can_include_external_project() {
            let sandbox = create_sandbox("projects");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("inputs").unwrap();
            let task = wg.get_task_from_project("inputs", "project").unwrap();

            let hasher_config = HasherConfig::default();
            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(
                get_input_files(result.inputs),
                [
                    "external/data.json",
                    "external/docs.md",
                    "external/moon.yml"
                ]
            );
        }

        #[tokio::test]
        async fn can_include_external_project_using_file_group() {
            let sandbox = create_sandbox("projects");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("inputs").unwrap();
            let task = wg.get_task_from_project("inputs", "projectGroup").unwrap();

            let hasher_config = HasherConfig::default();
            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(get_input_files(result.inputs), ["external/docs.md"]);
        }

        #[tokio::test]
        async fn can_include_external_project_using_filter_globs() {
            let sandbox = create_sandbox("projects");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let project = wg.get_project("inputs").unwrap();
            let task = wg.get_task_from_project("inputs", "projectFilter").unwrap();

            let hasher_config = HasherConfig::default();
            let result = generate_hash(&project, &task, &wg, &app, &hasher_config).await;

            assert_eq!(get_input_files(result.inputs), ["external/data.json"]);
        }
    }

    mod output_filtering {
        use super::*;

        #[tokio::test]
        async fn input_file_output_file() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inFileOutFile").unwrap();

            let expected = [
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "out/1",
                "out/3",
            ];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn input_file_output_dir() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inFileOutDir").unwrap();

            let expected = [".moon/toolchain.yml", ".moon/workspace.yml"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn input_file_output_glob() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inFileOutGlob").unwrap();

            let expected = [".moon/toolchain.yml", ".moon/workspace.yml"];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn input_glob_output_file() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inGlobOutFile").unwrap();

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
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn input_glob_output_dir() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inGlobOutDir").unwrap();

            let expected = [
                ".gitignore",
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "package.json",
            ];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }

        #[tokio::test]
        async fn input_glob_output_glob() {
            let sandbox = create_sandbox("output-filters");
            sandbox.enable_git();

            let (wg, app) = mock_workspace(sandbox.path()).await;
            let (vcs_config, glob_config) = create_hasher_configs();
            let project = wg.get_project("root").unwrap();
            let task = wg.get_task_from_project("root", "inGlobOutGlob").unwrap();

            let expected = [
                ".gitignore",
                ".moon/toolchain.yml",
                ".moon/workspace.yml",
                "package.json",
            ];

            // VCS
            let result = generate_hash(&project, &task, &wg, &app, &vcs_config).await;

            assert_eq!(get_input_files(result.inputs), expected);

            // Glob
            let result = generate_hash(&project, &task, &wg, &app, &glob_config).await;

            assert_eq!(get_input_files(result.inputs), expected);
        }
    }
}
