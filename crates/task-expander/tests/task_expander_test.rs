mod utils;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{InputPath, OutputPath, TaskArgs, TaskDependencyConfig};
use moon_task::Target;
use moon_task_expander::TaskExpander;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::env;
use utils::{create_context, create_project, create_project_with_tasks, create_task};

fn create_path_set(inputs: Vec<&str>) -> FxHashSet<WorkspaceRelativePathBuf> {
    FxHashSet::from_iter(inputs.into_iter().map(|s| s.into()))
}

mod task_expander {
    use super::*;

    #[test]
    fn doesnt_overlap_input_file() {
        let sandbox = create_sandbox("file-group");
        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.outputs.push(OutputPath::ProjectFile("out".into()));
        task.input_files.insert("project/source/out".into());

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_files.is_empty());
        assert_eq!(
            task.output_files,
            create_path_set(vec!["project/source/out"])
        );
    }

    #[test]
    fn doesnt_overlap_input_glob() {
        let sandbox = create_sandbox("file-group");
        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.outputs
            .push(OutputPath::ProjectGlob("out/**/*".into()));
        task.input_globs.insert("project/source/out/**/*".into());

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_globs.is_empty());
        assert_eq!(
            task.output_globs,
            create_path_set(vec!["project/source/out/**/*"])
        );
    }

    #[test]
    fn converts_dirs_to_globs() {
        let sandbox = create_sandbox("file-group");

        // Dir has to exist!
        sandbox.create_file("project/source/dir/file", "");

        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.inputs = vec![InputPath::ProjectFile("dir".into())];

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_files.is_empty());
        assert_eq!(
            task.input_globs,
            create_path_set(vec!["project/source/dir/**/*"])
        );
    }

    mod expand_command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @dirs(group) in task project:task cannot be used")]
        fn errors_on_token_funcs() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "@dirs(group)".into();

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_command(&mut task)
                .unwrap();
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "./$project/bin".into();

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_command(&mut task)
                .unwrap();

            assert_eq!(task.command, "./project/bin");
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "./$FOO/${BAR}/$BAZ_QUX".into();

            env::set_var("FOO", "foo");
            env::set_var("BAZ_QUX", "baz-qux");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_command(&mut task)
                .unwrap();

            env::remove_var("FOO");
            env::remove_var("BAZ_QUX");

            assert_eq!(task.command, "./foo/${BAR}/baz-qux");
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "./$FOO".into();
            task.env.insert("FOO".into(), "foo-self".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_command(&mut task)
                .unwrap();

            assert_eq!(task.command, "./foo-self");
        }
    }

    mod expand_args {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_args(&mut task)
                .unwrap();

            assert_eq!(
                task.args,
                [
                    "a",
                    "./config.yml",
                    "./dir/subdir/nested.json",
                    "./docs.md",
                    "./other/file.json",
                    "b"
                ]
            );
        }

        #[test]
        fn replaces_token_funcs_from_workspace_root() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];
            task.options.run_from_workspace_root = true;

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_args(&mut task)
                .unwrap();

            assert_eq!(
                task.args,
                [
                    "a",
                    "./project/source/config.yml",
                    "./project/source/dir/subdir/nested.json",
                    "./project/source/docs.md",
                    "./project/source/other/file.json",
                    "b"
                ]
            );
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec![
                "a".into(),
                "$project/dir".into(),
                "b".into(),
                "some/$task".into(),
            ];

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_args(&mut task)
                .unwrap();

            assert_eq!(task.args, ["a", "project/dir", "b", "some/task"]);
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec![
                "a".into(),
                "$FOO_BAR".into(),
                "b".into(),
                "c/${BAR_BAZ}/d".into(),
            ];

            env::set_var("BAR_BAZ", "bar-baz");
            env::set_var("FOO_BAR", "foo-bar");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_args(&mut task)
                .unwrap();

            env::remove_var("FOO_BAR");
            env::remove_var("BAR_BAZ");

            assert_eq!(task.args, ["a", "foo-bar", "b", "c/bar-baz/d"]);
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "${FOO_BAR}".into(), "b".into()];
            task.env.insert("FOO_BAR".into(), "foo-bar-self".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_args(&mut task)
                .unwrap();

            assert_eq!(task.args, ["a", "foo-bar-self", "b"]);
        }
    }

    mod expand_deps {
        use super::*;

        #[test]
        fn passes_args_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project_with_tasks(sandbox.path(), "project");
            let mut task = create_task();

            task.deps.push(TaskDependencyConfig {
                args: TaskArgs::String("a b c".into()),
                target: Target::parse("test").unwrap(),
                ..TaskDependencyConfig::default()
            });

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_deps(&mut task)
                .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig {
                    args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
                    target: Target::parse("~:test").unwrap(),
                    ..TaskDependencyConfig::default()
                }]
            );
        }

        #[test]
        fn supports_tokens_in_args() {
            let sandbox = create_empty_sandbox();
            let project = create_project_with_tasks(sandbox.path(), "project");
            let mut task = create_task();

            task.deps.push(TaskDependencyConfig {
                args: TaskArgs::String("$project $language".into()),
                target: Target::parse("test").unwrap(),
                ..TaskDependencyConfig::default()
            });

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_deps(&mut task)
                .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig {
                    args: TaskArgs::List(vec!["project".into(), "unknown".into()]),
                    target: Target::parse("~:test").unwrap(),
                    ..TaskDependencyConfig::default()
                }]
            );
        }

        #[test]
        fn passes_env_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project_with_tasks(sandbox.path(), "project");
            let mut task = create_task();

            task.deps.push(TaskDependencyConfig {
                env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                target: Target::parse("test").unwrap(),
                ..TaskDependencyConfig::default()
            });

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_deps(&mut task)
                .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                    target: Target::parse("~:test").unwrap(),
                    optional: None,
                }]
            );
        }

        #[test]
        fn supports_token_in_env() {
            let sandbox = create_empty_sandbox();
            let project = create_project_with_tasks(sandbox.path(), "project");
            let mut task = create_task();

            task.deps.push(TaskDependencyConfig {
                env: FxHashMap::from_iter([("FOO".into(), "$project-$language".into())]),
                target: Target::parse("test").unwrap(),
                ..TaskDependencyConfig::default()
            });

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_deps(&mut task)
                .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([("FOO".into(), "project-unknown".into())]),
                    target: Target::parse("~:test").unwrap(),
                    optional: None,
                }]
            );
        }

        #[test]
        fn passes_args_and_env_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project_with_tasks(sandbox.path(), "project");
            let mut task = create_task();

            task.deps.push(TaskDependencyConfig {
                args: TaskArgs::String("a b c".into()),
                env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                target: Target::parse("test").unwrap(),
                optional: None,
            });

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_deps(&mut task)
                .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig {
                    args: TaskArgs::List(vec!["a".into(), "b".into(), "c".into()]),
                    env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                    target: Target::parse("~:test").unwrap(),
                    optional: None,
                }]
            );
        }
    }

    mod expand_env {
        use super::*;

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "inner-${FOO}".into());
            task.env.insert("KEY3".into(), "$KEY1-self".into());

            env::set_var("FOO", "foo");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            env::remove_var("FOO");

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "inner-foo".into()),
                    ("KEY3".into(), "value1-self".into()),
                ])
            );
        }

        #[test]
        fn replaces_tokens() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "@globs(all)".into());
            task.env.insert("KEY2".into(), "$project-$task".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "./*.md,./**/*.json".into()),
                    ("KEY2".into(), "project-task".into()),
                ])
            );
        }

        #[test]
        fn replaces_tokens_from_workspace_root() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.run_from_workspace_root = true;

            task.env.insert("KEY1".into(), "@globs(all)".into());
            task.env.insert("KEY2".into(), "$project-$task".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    (
                        "KEY1".into(),
                        "./project/source/*.md,./project/source/**/*.json".into()
                    ),
                    ("KEY2".into(), "project-task".into()),
                ])
            );
        }

        #[test]
        fn can_use_env_vars_and_token_vars() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env
                .insert("KEY".into(), "$project-$FOO-$unknown".into());

            env::set_var("FOO", "foo");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            env::remove_var("FOO");

            assert_eq!(
                task.env,
                FxHashMap::from_iter([("KEY".into(), "project-foo-$unknown".into()),])
            );
        }

        #[test]
        fn loads_from_env_file() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_files = Some(vec![InputPath::ProjectFile(".env".into())]);

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()), // Not overridden by env file
                    ("KEY3".into(), "value3".into()),
                ])
            );
        }

        #[test]
        fn loads_from_root_env_file_and_substitutes() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.env_files = Some(vec![InputPath::WorkspaceFile(".env-shared".into())]);

            env::set_var("EXTERNAL", "external-value");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            env::remove_var("EXTERNAL");

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("ROOT".into(), "true".into()),
                    ("BASE".into(), "value".into()),
                    ("FROM_SELF1".into(), "value".into()),
                    ("FROM_SELF2".into(), "value".into()),
                    ("FROM_SYSTEM".into(), "external-value".into()),
                ])
            );
        }

        #[test]
        fn can_substitute_var_from_env_file() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.env_files = Some(vec![InputPath::WorkspaceFile(".env-shared".into())]);
            task.env.insert("TOP_LEVEL".into(), "$BASE".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(task.env.get("TOP_LEVEL").unwrap(), "value");
        }

        #[test]
        fn can_substitute_self_from_system() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            env::set_var("MYPATH", "/another/path");

            let mut task = create_task();
            task.env.insert("MYPATH".into(), "/path:$MYPATH".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(task.env.get("MYPATH").unwrap(), "/path:/another/path");

            env::remove_var("MYPATH");
        }

        #[test]
        fn doesnt_substitute_self_from_local() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("MYPATH".into(), "/path:$MYPATH".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(task.env.get("MYPATH").unwrap(), "/path:$MYPATH");
        }

        #[test]
        fn loads_from_multiple_env_file() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_files = Some(vec![
                InputPath::ProjectFile(".env".into()),
                InputPath::WorkspaceFile(".env-shared".into()),
            ]);

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()), // Not overridden by env file
                    ("KEY3".into(), "value3".into()),
                    // shared
                    ("ROOT".into(), "true".into()),
                    ("BASE".into(), "value".into()),
                    ("FROM_SELF1".into(), "value".into()),
                    ("FROM_SELF2".into(), "value".into()),
                    ("FROM_SYSTEM".into(), "".into()),
                ])
            );
        }

        #[test]
        fn skips_missing_env_file() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_files = Some(vec![InputPath::ProjectFile(".env-missing".into())]);

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    ("KEY1".into(), "value1".into()),
                    ("KEY2".into(), "value2".into()),
                ])
            );
        }

        #[test]
        #[should_panic(expected = "Failed to parse env file")]
        fn errors_invalid_env_file() {
            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.env_files = Some(vec![InputPath::ProjectFile(".env-invalid".into())]);

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();
        }
    }

    mod expand_inputs {
        use super::*;

        #[test]
        fn extracts_env_var() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::EnvVar("FOO_BAR".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(task.input_env, FxHashSet::from_iter(["FOO_BAR".into()]));
            assert_eq!(task.input_globs, FxHashSet::default());
            assert_eq!(task.input_files, FxHashSet::default());
        }

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@files(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(task.input_globs, FxHashSet::default());
            assert_eq!(
                task.input_files,
                create_path_set(vec![
                    "project/source/dir/subdir/nested.json",
                    "project/source/file.txt",
                    "project/source/docs.md",
                    "project/source/config.yml",
                    "project/source/other/file.json"
                ])
            );
        }

        #[test]
        fn splits_token_func_into_files_globs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@group(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.input_files,
                create_path_set(vec![
                    "project/source/dir/subdir",
                    "project/source/file.txt",
                    "project/source/config.yml",
                ])
            );
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs
                .push(InputPath::ProjectGlob("$task/**/*".into()));
            task.inputs
                .push(InputPath::WorkspaceFile("$project/index.js".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(task.input_files, create_path_set(vec!["project/index.js"]));
        }
    }

    mod expand_outputs {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@files(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(task.output_globs, FxHashSet::default());
            assert_eq!(
                task.output_files,
                create_path_set(vec![
                    "project/source/dir/subdir/nested.json",
                    "project/source/file.txt",
                    "project/source/docs.md",
                    "project/source/config.yml",
                    "project/source/other/file.json"
                ])
            );
        }

        #[test]
        fn splits_token_func_into_files_globs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@group(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.output_files,
                create_path_set(vec![
                    "project/source/dir/subdir",
                    "project/source/file.txt",
                    "project/source/config.yml",
                ])
            );
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectGlob("$task/**/*".into()));
            task.outputs
                .push(OutputPath::WorkspaceFile("$project/index.js".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(task.output_files, create_path_set(vec!["project/index.js"]));
        }
    }
}
