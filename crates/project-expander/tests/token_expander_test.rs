mod utils;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{InputPath, LanguageType, OutputPath, ProjectType};
use moon_project_expander::{ExpandedResult, TokenExpander};
use rustc_hash::FxHashMap;
use starbase_sandbox::{create_empty_sandbox, create_sandbox, predicates::prelude::*};
use std::borrow::Cow;
use std::env;
use utils::{create_context, create_project, create_task};

mod token_expander {
    use super::*;

    #[test]
    #[should_panic(expected = "Unknown token @unknown(id).")]
    fn errors_for_unknown_token_func() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@unknown(id)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown file group unknown used in token @files(unknown).")]
    fn errors_for_unknown_file_group() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@files(unknown)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Token @in(str) received an invalid type for index \"str\"")]
    fn errors_for_invalid_in_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@in(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Input index 10 does not exist for token @in(10).")]
    fn errors_for_invalid_in_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@in(10)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Token @out(str) received an invalid type for index \"str\"")]
    fn errors_for_invalid_out_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@out(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Output index 10 does not exist for token @out(10).")]
    fn errors_for_invalid_out_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(&project, sandbox.path());
        let expander = TokenExpander::new(&context);

        expander.replace_function(&task, "@out(10)").unwrap();
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

            let context = create_context(&project, sandbox.path());
            let expander = TokenExpander::new(&context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language"))
                    .unwrap(),
                "javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$project"))
                    .unwrap(),
                "project"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectAlias"))
                    .unwrap(),
                ""
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectSource"))
                    .unwrap(),
                "project/source"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectRoot"))
                    .unwrap(),
                project.root.to_string_lossy()
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectType"))
                    .unwrap(),
                "library"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$target"))
                    .unwrap(),
                "project:task"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$task"))
                    .unwrap(),
                "task"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$taskPlatform"))
                    .unwrap(),
                "unknown"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$taskType"))
                    .unwrap(),
                "test"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$workspaceRoot"))
                    .unwrap(),
                sandbox.path().to_string_lossy()
            );

            assert!(predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}")
                .unwrap()
                .eval(
                    &expander
                        .replace_variable(&task, Cow::Borrowed("$date"))
                        .unwrap()
                ));

            assert!(predicate::str::is_match("[0-9]{2}:[0-9]{2}:[0-9]{2}")
                .unwrap()
                .eval(
                    &expander
                        .replace_variable(&task, Cow::Borrowed("$time"))
                        .unwrap()
                ));

            assert!(predicate::str::is_match(
                "[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}:[0-9]{2}:[0-9]{2}"
            )
            .unwrap()
            .eval(
                &expander
                    .replace_variable(&task, Cow::Borrowed("$datetime"))
                    .unwrap()
            ));

            assert!(predicate::str::is_match("[0-9]{10}").unwrap().eval(
                &expander
                    .replace_variable(&task, Cow::Borrowed("$timestamp"))
                    .unwrap()
            ));
        }

        #[test]
        fn replaces_variable_at_different_positions() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.language = LanguageType::JavaScript;
            let task = create_task();

            let context = create_context(&project, sandbox.path());
            let expander = TokenExpander::new(&context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language"))
                    .unwrap(),
                "javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language/before"))
                    .unwrap(),
                "javascript/before"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("after/$language"))
                    .unwrap(),
                "after/javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("in/$language/between"))
                    .unwrap(),
                "in/javascript/between"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("partof$languagestring"))
                    .unwrap(),
                "partofjavascriptstring"
            );
        }

        #[test]
        fn keeps_unknown_var_asis() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let task = create_task();

            let context = create_context(&project, sandbox.path());
            let expander = TokenExpander::new(&context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$unknown"))
                    .unwrap(),
                "$unknown"
            );
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

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_command(&task).unwrap();
        }

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "bin".into();

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(expander.expand_command(&task).unwrap(), "bin");
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin".into();

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(expander.expand_command(&task).unwrap(), "project/bin");
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin/$task".into();

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(expander.expand_command(&task).unwrap(), "project/bin/task");
        }
    }

    mod envs {
        use super::*;

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("KEY".into(), "value".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([("KEY".into(), "value".into())])
            );
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("VAR".into(), "$project-prod".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([("VAR".into(), "project-prod".into())])
            );
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env
                .insert("VARS".into(), "$project-debug-$task".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([("VARS".into(), "project-debug-task".into())])
            );
        }

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GROUP".into(), "@group(all)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([("GROUP".into(), "project/source/config.yml,project/source/dir/subdir,project/source/*.md,project/source/**/*.json".into())])
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("DIRS".into(), "@dirs(dirs)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([(
                    "DIRS".into(),
                    "project/source/dir/subdir,project/source/other".into()
                )])
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("FILES".into(), "@files(all)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([(
                    "FILES".into(),
                    "project/source/config.yml,project/source/dir/subdir/nested.json,project/source/docs.md,project/source/other/file.json".into()
                )])
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GLOBS".into(), "@globs(all)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([(
                    "GLOBS".into(),
                    "project/source/*.md,project/source/**/*.json".into()
                )])
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("ROOT".into(), "@root(all)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_env(&task).unwrap(),
                FxHashMap::from_iter([("ROOT".into(), "project/source/dir/subdir".into())])
            );
        }

        #[test]
        #[should_panic(expected = "Token @in(0) cannot be used within task env.")]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("IN".into(), "@in(0)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_env(&task).unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @out(0) cannot be used within task env.")]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("OUT".into(), "@out(0)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_env(&task).unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @envs(envs) cannot be used within task env.")]
        fn errors_for_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("OUT".into(), "@envs(envs)".into());

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_env(&task).unwrap();
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::EnvVar("FOO_BAR".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    env: vec!["FOO_BAR".into()],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_env_var_glob() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::EnvVarGlob("FOO_*".into())];

            env::set_var("FOO_ONE", "1");
            env::set_var("FOO_TWO", "2");
            env::set_var("FOO_THREE", "3");
            env::set_var("BAR_ONE", "1");

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            let mut result = expander.expand_inputs(&task).unwrap();
            result.env.sort();

            assert_eq!(
                result,
                ExpandedResult {
                    env: vec!["FOO_ONE".into(), "FOO_THREE".into(), "FOO_TWO".into()],
                    ..ExpandedResult::default()
                }
            );

            env::remove_var("FOO_ONE");
            env::remove_var("FOO_TWO");
            env::remove_var("FOO_THREE");
            env::remove_var("BAR_ONE");
        }

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@group(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@dirs(dirs)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir"),
                        WorkspaceRelativePathBuf::from("project/source/other"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@files(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir/nested.json"),
                        WorkspaceRelativePathBuf::from("project/source/docs.md"),
                        WorkspaceRelativePathBuf::from("project/source/other/file.json"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@globs(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@root(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![WorkspaceRelativePathBuf::from("project/source/dir/subdir")],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@envs(envs)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    env: vec!["FOO_BAR".into()],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        #[should_panic(expected = "Token @in(0) cannot be used within task inputs.")]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@in(0)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_inputs(&task).unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @out(0) cannot be used within task inputs.")]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@out(0)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_inputs(&task).unwrap();
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

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/project:task"),
                        WorkspaceRelativePathBuf::from("project/source/unknown"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
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

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/project:task/file.txt"),
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/project:task/files/**/*"),
                    ],
                    ..ExpandedResult::default()
                }
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

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![],
                    globs: vec![WorkspaceRelativePathBuf::from("project/source/dir/**/*"),],
                    ..ExpandedResult::default()
                }
            );
        }
    }

    mod outputs {
        use super::*;

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@group(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@dirs(dirs)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir"),
                        WorkspaceRelativePathBuf::from("project/source/other"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@files(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir/nested.json"),
                        WorkspaceRelativePathBuf::from("project/source/docs.md"),
                        WorkspaceRelativePathBuf::from("project/source/other/file.json"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@globs(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@root(all)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![WorkspaceRelativePathBuf::from("project/source/dir/subdir")],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        #[should_panic(expected = "Token @in(0) cannot be used within task outputs.")]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@in(0)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @out(0) cannot be used within task outputs.")]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@out(0)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        #[should_panic(expected = "Token @envs(envs) cannot be used within task outputs.")]
        fn errors_for_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@envs(envs)".into())];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        fn converts_variables() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![
                OutputPath::ProjectFile("$task/file.txt".into()),
                OutputPath::ProjectGlob("$task/files/**/*".into()),
                OutputPath::WorkspaceFile("cache/$target/file.txt".into()),
                OutputPath::WorkspaceGlob("cache/$target/files/**/*".into()),
            ];

            let context = create_context(&project, sandbox.path());
            let mut expander = TokenExpander::new(&context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/project:task/file.txt"),
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/project:task/files/**/*"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }
    }
}
