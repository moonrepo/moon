mod utils;

use moon_config::{
    Input, Output, TaskArgs, TaskDependencyConfig, schematic::RegexSetting, test_utils::*,
};
use moon_env_var::GlobalEnvBag;
use moon_task::{Target, TaskFileInput, TaskGlobInput};
use moon_task_expander::TaskExpander;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use utils::*;

mod task_expander {
    use super::*;

    #[test]
    fn doesnt_overlap_input_file() {
        let sandbox = create_sandbox("file-group");
        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.outputs.push(Output::File(stub_file_output("out")));
        task.input_files
            .insert("project/source/out".into(), TaskFileInput::default());

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_files.is_empty());
        assert_eq!(
            task.output_files,
            create_file_output_map(vec!["project/source/out"])
        );
    }

    #[test]
    fn doesnt_overlap_input_glob() {
        let sandbox = create_sandbox("file-group");
        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.outputs
            .push(Output::Glob(stub_glob_output("out/**/*")));
        task.input_globs
            .insert("project/source/out/**/*".into(), TaskGlobInput::default());

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_globs.is_empty());
        assert_eq!(
            task.output_globs,
            create_glob_output_map(vec!["project/source/out/**/*"])
        );
    }

    #[test]
    fn converts_dirs_to_globs() {
        let sandbox = create_sandbox("file-group");

        // Dir has to exist!
        sandbox.create_file("project/source/dir/file", "");

        let project = create_project(sandbox.path());

        let mut task = create_task();
        task.inputs = vec![Input::parse("dir").unwrap()];

        let context = create_context(sandbox.path());
        let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

        assert!(task.input_files.is_empty());
        assert_eq!(
            task.input_globs,
            create_glob_input_map(vec!["project/source/dir/**/*"])
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

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_command(&mut task)
                .unwrap();

            assert_eq!(task.command, "./$FOO/${BAR}/$BAZ_QUX");

            assert!(task.input_env.contains("FOO"));
            assert!(task.input_env.contains("BAR"));
            assert!(task.input_env.contains("BAZ_QUX"));
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

            assert_eq!(task.command, "./$FOO");
            assert!(task.input_env.contains("FOO"));
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

            let context = create_context(sandbox.path());
            let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

            assert_eq!(task.args, ["a", "$FOO_BAR", "b", "c/${BAR_BAZ}/d"]);

            assert!(task.input_env.contains("FOO_BAR"));
            assert!(task.input_env.contains("BAR_BAZ"));
        }

        #[test]
        fn replaces_env_vars_from_file() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file(".env", "FOO_BAR=foo-bar\nBAR_BAZ=bar-baz");

            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.env_files = Some(vec![Input::parse("/.env").unwrap()]);
            task.args = vec![
                "a".into(),
                "$FOO_BAR".into(),
                "b".into(),
                "c/${BAR_BAZ}/d".into(),
            ];

            let context = create_context(sandbox.path());
            let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

            assert_eq!(task.args, ["a", "$FOO_BAR", "b", "c/${BAR_BAZ}/d"]);

            assert!(task.input_env.contains("FOO_BAR"));
            assert!(task.input_env.contains("BAR_BAZ"));
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "${FOO_BAR}".into(), "b".into()];
            task.env.insert("FOO_BAR".into(), "foo-bar-self".into());

            let context = create_context(sandbox.path());
            let task = TaskExpander::new(&project, &context).expand(&task).unwrap();

            assert_eq!(task.args, ["a", "${FOO_BAR}", "b"]);
            assert!(task.input_env.contains("FOO_BAR"));
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

            let bag = GlobalEnvBag::instance();
            bag.set("FOO", "foo");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            bag.remove("FOO");

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

            let bag = GlobalEnvBag::instance();
            bag.set("FOO", "foo");

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            bag.remove("FOO");

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
            task.options.env_files = Some(vec![Input::parse(".env").unwrap()]);

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
            use std::env;

            let sandbox = create_sandbox("env-file");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.options.env_files = Some(vec![Input::parse("/.env-shared").unwrap()]);

            // dotenvy operates on actual env
            unsafe { env::set_var("EXTERNAL", "external-value") };

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            unsafe { env::remove_var("EXTERNAL") };

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
            task.options.env_files = Some(vec![Input::parse("/.env-shared").unwrap()]);
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

            let bag = GlobalEnvBag::instance();
            bag.set("MYPATH", "/another/path");

            let mut task = create_task();
            task.env.insert("MYPATH".into(), "/path:$MYPATH".into());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();

            assert_eq!(task.env.get("MYPATH").unwrap(), "/path:/another/path");

            bag.remove("MYPATH");
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
                Input::parse(".env").unwrap(),
                Input::parse("/.env-shared").unwrap(),
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
            task.options.env_files = Some(vec![Input::parse(".env-missing").unwrap()]);

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
            task.options.env_files = Some(vec![Input::parse(".env-invalid").unwrap()]);

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_env(&mut task)
                .unwrap();
        }
    }

    mod expand_inputs {
        use super::*;

        #[test]
        fn inherits_file_input_params() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs
                .push(Input::parse("file://a.txt?optional").unwrap());
            task.inputs
                .push(Input::parse("file://dir/b.txt?content=a|b|c").unwrap());
            task.inputs
                .push(Input::parse("file:///root/c.txt?optional=false&content=a|b|c").unwrap());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_files,
                FxHashMap::from_iter([
                    (
                        "project/source/a.txt".into(),
                        TaskFileInput {
                            optional: Some(true),
                            ..Default::default()
                        }
                    ),
                    (
                        "project/source/dir/b.txt".into(),
                        TaskFileInput {
                            content: Some(RegexSetting::try_from("a|b|c".to_owned()).unwrap()),
                            ..Default::default()
                        }
                    ),
                    (
                        "root/c.txt".into(),
                        TaskFileInput {
                            content: Some(RegexSetting::try_from("a|b|c".to_owned()).unwrap()),
                            optional: Some(false),
                        }
                    ),
                ])
            );
        }

        #[test]
        fn inherits_glob_input_params() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(Input::parse("glob://a.*?cache").unwrap());
            task.inputs
                .push(Input::parse("glob://dir/b.*?cache=false").unwrap());
            task.inputs
                .push(Input::parse("glob:///root/c.*?cache=true").unwrap());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_globs,
                FxHashMap::from_iter([
                    ("project/source/a.*".into(), TaskGlobInput { cache: true }),
                    (
                        "project/source/dir/b.*".into(),
                        TaskGlobInput { cache: false }
                    ),
                    ("root/c.*".into(), TaskGlobInput { cache: true }),
                ])
            );
        }

        #[test]
        fn extracts_env_var() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(Input::EnvVar("FOO_BAR".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(task.input_env, FxHashSet::from_iter(["FOO_BAR".into()]));
            assert_eq!(task.input_globs, FxHashMap::default());
            assert_eq!(task.input_files, FxHashMap::default());
        }

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(Input::parse("file.txt").unwrap());
            task.inputs.push(Input::TokenFunc("@files(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(task.input_globs, FxHashMap::default());
            assert_eq!(
                task.input_files,
                create_file_input_map(vec![
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
            task.inputs.push(Input::parse("file.txt").unwrap());
            task.inputs.push(Input::TokenFunc("@group(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_globs,
                create_glob_input_map(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.input_files,
                create_file_input_map(vec![
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
            task.inputs.push(Input::parse("$task/**/*").unwrap());
            task.inputs
                .push(Input::parse("/$project/index.js").unwrap());

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(
                task.input_globs,
                create_glob_input_map(vec!["project/source/task/**/*"])
            );
            assert_eq!(
                task.input_files,
                create_file_input_map(vec!["project/index.js"])
            );
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
                .push(Output::File(stub_file_output("file.txt")));
            task.outputs.push(Output::TokenFunc("@files(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(task.output_globs, FxHashMap::default());
            assert_eq!(
                task.output_files,
                create_file_output_map(vec![
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
                .push(Output::File(stub_file_output("file.txt")));
            task.outputs.push(Output::TokenFunc("@group(all)".into()));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(
                task.output_globs,
                create_glob_output_map(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.output_files,
                create_file_output_map(vec![
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
                .push(Output::Glob(stub_glob_output("$task/**/*")));
            task.outputs
                .push(Output::File(stub_file_output("/$project/index.js")));

            let context = create_context(sandbox.path());
            TaskExpander::new(&project, &context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(
                task.output_globs,
                create_glob_output_map(vec!["project/source/task/**/*"])
            );
            assert_eq!(
                task.output_files,
                create_file_output_map(vec!["project/index.js"])
            );
        }
    }
}
