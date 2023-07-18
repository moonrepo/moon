use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{InputPath, LanguageType, ProjectType};
use moon_project::{FileGroup, Project};
use moon_project_graph2::TokenExpander;
use moon_task::{Target, Task};
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox, predicates::prelude::*};
use std::path::Path;

fn create_project(workspace_root: &Path) -> Project {
    let source = WorkspaceRelativePathBuf::from("project/source");

    Project {
        id: Id::raw("project"),
        root: workspace_root.join(source.as_str()),
        file_groups: FxHashMap::from_iter([
            (
                "all".into(),
                FileGroup::new_with_source(
                    "all",
                    [
                        source.join("*.md"),
                        source.join("**/*.json"),
                        source.join("config.yml"),
                        source.join("dir/subdir"),
                    ],
                )
                .unwrap(),
            ),
            (
                "dirs".into(),
                FileGroup::new_with_source(
                    "dirs",
                    [
                        source.join("other"),
                        source.join("dir/*"),
                        source.join("**/*.md"),
                    ],
                )
                .unwrap(),
            ),
        ]),
        source,
        ..Project::default()
    }
}

fn create_task() -> Task {
    Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Task::default()
    }
}

mod token_expander {
    use super::*;

    #[test]
    #[should_panic(expected = "Unknown token @unknown(id).")]
    fn errors_for_unknown_token_func() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@unknown(id)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown file group unknown used in token @files(unknown).")]
    fn errors_for_unknown_file_group() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@files(unknown)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Token @in(str) received an invalid type for index \"str\"")]
    fn errors_for_invalid_in_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@in(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Input index 10 does not exist for token @in(10).")]
    fn errors_for_invalid_in_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@in(10)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Token @out(str) received an invalid type for index \"str\"")]
    fn errors_for_invalid_out_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@out(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Output index 10 does not exist for token @out(10).")]
    fn errors_for_invalid_out_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let expander = TokenExpander::for_args(&project, &task, sandbox.path());

        expander.replace_function("@out(10)").unwrap();
    }

    mod vars {
        use super::*;
        #[test]
        fn replaces_variables() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.type_of = ProjectType::Library;
            project.language = LanguageType::JavaScript;

            let task = create_task();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(
                expander.replace_variable("$language").unwrap(),
                "javascript"
            );
            assert_eq!(expander.replace_variable("$project").unwrap(), "project");
            assert_eq!(expander.replace_variable("$projectAlias").unwrap(), "");
            assert_eq!(
                expander.replace_variable("$projectSource").unwrap(),
                "project/source"
            );
            assert_eq!(
                expander.replace_variable("$projectRoot").unwrap(),
                project.root.to_string_lossy()
            );
            assert_eq!(
                expander.replace_variable("$projectType").unwrap(),
                "library"
            );
            assert_eq!(
                expander.replace_variable("$target").unwrap(),
                "project:task"
            );
            assert_eq!(expander.replace_variable("$task").unwrap(), "task");
            assert_eq!(
                expander.replace_variable("$taskPlatform").unwrap(),
                "unknown"
            );
            assert_eq!(expander.replace_variable("$taskType").unwrap(), "test");
            assert_eq!(
                expander.replace_variable("$workspaceRoot").unwrap(),
                sandbox.path().to_string_lossy()
            );

            assert!(predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}")
                .unwrap()
                .eval(&expander.replace_variable("$date").unwrap()));

            assert!(predicate::str::is_match("[0-9]{2}:[0-9]{2}:[0-9]{2}")
                .unwrap()
                .eval(&expander.replace_variable("$time").unwrap()));

            assert!(predicate::str::is_match(
                "[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}:[0-9]{2}:[0-9]{2}"
            )
            .unwrap()
            .eval(&expander.replace_variable("$datetime").unwrap()));

            assert!(predicate::str::is_match("[0-9]{10}")
                .unwrap()
                .eval(&expander.replace_variable("$timestamp").unwrap()));
        }

        #[test]
        fn replaces_variable_at_different_positions() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.language = LanguageType::JavaScript;
            let task = create_task();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(
                expander.replace_variable("$language").unwrap(),
                "javascript"
            );
            assert_eq!(
                expander.replace_variable("$language/before").unwrap(),
                "javascript/before"
            );
            assert_eq!(
                expander.replace_variable("after/$language").unwrap(),
                "after/javascript"
            );
            assert_eq!(
                expander.replace_variable("in/$language/between").unwrap(),
                "in/javascript/between"
            );
            assert_eq!(
                expander.replace_variable("partof$languagestring").unwrap(),
                "partofjavascriptstring"
            );
        }

        #[test]
        fn keeps_unknown_var_asis() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let task = create_task();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(expander.replace_variable("$unknown").unwrap(), "$unknown");
        }
    }

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @files(sources) cannot be used within task commands.")]
        fn errors_for_func() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "@files(sources)".into();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            expander.expand_command().unwrap();
        }

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "bin".into();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(expander.expand_command().unwrap(), "bin");
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin".into();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(expander.expand_command().unwrap(), "project/bin");
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin/$task".into();

            let expander = TokenExpander::for_command(&project, &task, sandbox.path());

            assert_eq!(expander.expand_command().unwrap(), "project/bin/task");
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn skips_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::EnvVar("FOO_BAR".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(expander.expand_inputs().unwrap(), (vec![], vec![]));
        }

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@group(all)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ]
                )
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@dirs(dirs)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/other"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    vec![]
                )
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@files(all)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/docs.md"),
                        WorkspaceRelativePathBuf::from("project/source/other/file.json"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir/nested.json"),
                    ],
                    vec![]
                )
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@globs(all)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![],
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ]
                )
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@root(all)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![WorkspaceRelativePathBuf::from("project/source/dir/subdir")],
                    vec![]
                )
            );
        }

        #[test]
        #[should_panic(expected = "Token @in(0) cannot be used within task inputs.")]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@in(0)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            expander.expand_inputs().unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @out(0) cannot be used within task inputs.")]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@out(0)".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            expander.expand_inputs().unwrap();
        }

        #[test]
        fn supports_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![
                InputPath::TokenVar("$target".into()),
                InputPath::TokenVar("$taskPlatform".into()),
            ];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/project:task"),
                        WorkspaceRelativePathBuf::from("project/source/unknown"),
                    ],
                    vec![]
                )
            );
        }

        #[test]
        fn supports_vars_in_paths() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![
                InputPath::ProjectFile("$task/file.txt".into()),
                InputPath::ProjectGlob("$task/files/**/*".into()),
                InputPath::WorkspaceFile("cache/$target/file.txt".into()),
                InputPath::WorkspaceGlob("cache/$target/files/**/*".into()),
            ];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/task/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/project:task/file.txt"),
                    ],
                    vec![
                        WorkspaceRelativePathBuf::from("project/source/task/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/project:task/files/**/*"),
                    ]
                )
            );
        }

        #[test]
        fn converts_dirs_to_globs() {
            let sandbox = create_empty_sandbox();

            // Dir has to exist!
            sandbox.create_file("project/source/dir/file", "");

            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::ProjectFile("dir".into())];

            let expander = TokenExpander::for_inputs(&project, &task, sandbox.path());

            assert_eq!(
                expander.expand_inputs().unwrap(),
                (
                    vec![],
                    vec![WorkspaceRelativePathBuf::from("project/source/dir/**/*"),]
                )
            );
        }
    }
}
