mod utils;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{InputPath, OutputPath};
use moon_project::Project;
use moon_task::Target;
use moon_task_expander::TasksExpander;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use utils::{create_project, create_project_with_tasks, create_task};

fn create_path_set(inputs: Vec<&str>) -> FxHashSet<WorkspaceRelativePathBuf> {
    FxHashSet::from_iter(inputs.into_iter().map(|s| s.into()))
}

mod tasks_expander {
    use super::*;

    mod expand_command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @dirs(group) cannot be used within task commands.")]
        fn errors_on_token_funcs() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "@dirs(group)".into();

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "./$project/bin".into();

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();

            assert_eq!(task.command, "./project/bin");
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "./$FOO/${BAR}/$BAZ_QUX".into();

            env::set_var("FOO", "foo");
            env::set_var("BAZ_QUX", "baz-qux");

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_command(&mut task)
            .unwrap();

            env::remove_var("FOO");
            env::remove_var("BAZ_QUX");

            assert_eq!(task.command, "./foo//baz-qux");
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "./$FOO".into();
            task.env.insert("FOO".into(), "foo-self".into());

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_args(&mut task)
            .unwrap();

            assert_eq!(
                task.args,
                [
                    "a",
                    "./config.yml",
                    "./docs.md",
                    "./other/file.json",
                    "./dir/subdir/nested.json",
                    "b"
                ]
            );
        }

        #[test]
        fn replaces_token_funcs_from_workspace_root() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];
            task.options.run_from_workspace_root = true;

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_args(&mut task)
            .unwrap();

            assert_eq!(
                task.args,
                [
                    "a",
                    "./project/source/config.yml",
                    "./project/source/docs.md",
                    "./project/source/other/file.json",
                    "./project/source/dir/subdir/nested.json",
                    "b"
                ]
            );
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.args = vec![
                "a".into(),
                "$project/dir".into(),
                "b".into(),
                "some/$task".into(),
            ];

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_args(&mut task)
            .unwrap();

            assert_eq!(task.args, ["a", "project/dir", "b", "some/task"]);
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.args = vec![
                "a".into(),
                "$FOO_BAR".into(),
                "b".into(),
                "c/${BAR_BAZ}/d".into(),
            ];

            env::set_var("FOO_BAR", "foo-bar");
            env::set_var("BAR_BAZ", "bar-baz");

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_args(&mut task)
            .unwrap();

            env::remove_var("FOO_BAR");
            env::remove_var("BAR_BAZ");

            assert_eq!(task.args, ["a", "foo-bar", "b", "c/bar-baz/d"]);
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.args = vec!["a".into(), "${FOO_BAR}".into(), "b".into()];
            task.env.insert("FOO_BAR".into(), "foo-bar-self".into());

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_args(&mut task)
            .unwrap();

            assert_eq!(task.args, ["a", "foo-bar-self", "b"]);
        }
    }

    mod expand_deps {
        use super::*;

        fn no_query(_: String) -> miette::Result<Vec<Arc<Project>>> {
            Ok(vec![])
        }

        fn do_query(_: String) -> miette::Result<Vec<Arc<Project>>> {
            let root = PathBuf::from("/root");

            Ok(vec![
                Arc::new(create_project_with_tasks(&root, "foo")),
                Arc::new(create_project_with_tasks(&root, "bar")),
                Arc::new(create_project_with_tasks(&root, "baz")),
            ])
        }

        mod all {
            use super::*;

            #[test]
            #[should_panic(
                expected = "Invalid dependency :build for project:task. All (:) scope is not"
            )]
            fn errors() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse(":build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();
            }
        }

        mod deps {
            use super::*;

            #[test]
            fn no_depends_on() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("^:build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query) // Tested here
                .unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn returns_tasks_of_same_name() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("^:build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        Target::parse("foo:build").unwrap(),
                        Target::parse("bar:build").unwrap(),
                        Target::parse("baz:build").unwrap()
                    ]
                );
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task foo:dev."
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.options.persistent = false;
                task.deps.push(Target::parse("^:dev").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();
            }
        }

        mod own_self {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("~:build").unwrap());
                task.deps.push(Target::parse("lint").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        Target::parse("project:build").unwrap(),
                        Target::parse("project:lint").unwrap()
                    ]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("task").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn ignores_dupes() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("~:test").unwrap());
                task.deps.push(Target::parse("test").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();

                assert_eq!(task.deps, vec![Target::parse("project:test").unwrap()]);
            }

            #[test]
            #[should_panic(
                expected = "Invalid dependency project:unknown for project:task, target does not"
            )]
            fn errors_unknown_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("~:unknown").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.options.persistent = false;
                task.deps.push(Target::parse("~:dev").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();
            }
        }

        mod project {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("project:build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();

                assert_eq!(task.deps, vec![Target::parse("project:build").unwrap()]);
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("project:task").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn refs_other_project_tasks() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("foo:build").unwrap());
                task.deps.push(Target::parse("bar:lint").unwrap());
                task.deps.push(Target::parse("baz:test").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, |input| {
                    Ok(vec![Arc::new(create_project_with_tasks(
                        sandbox.path(),
                        if input.contains("foo") {
                            "foo"
                        } else if input.contains("bar") {
                            "bar"
                        } else {
                            "baz"
                        },
                    ))])
                })
                .unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        Target::parse("foo:build").unwrap(),
                        Target::parse("bar:lint").unwrap(),
                        Target::parse("baz:test").unwrap()
                    ]
                );
            }

            #[test]
            #[should_panic(
                expected = "Invalid dependency foo:unknown for project:task, target does not exist."
            )]
            fn errors_unknown_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("foo:unknown").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query)
                .unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.options.persistent = false;
                task.deps.push(Target::parse("foo:dev").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();
            }
        }

        mod tag {
            use super::*;

            #[test]
            fn no_tags() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("#tag:build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, no_query) // Tested here
                .unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn returns_tasks_of_same_name() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("#tag:build").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        Target::parse("foo:build").unwrap(),
                        Target::parse("bar:build").unwrap(),
                        Target::parse("baz:build").unwrap()
                    ]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.deps.push(Target::parse("#tag:task").unwrap());

                let cloned_project = Arc::new(project.clone());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, move |_| Ok(vec![cloned_project.clone()]))
                .unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let mut task = create_task();

                task.options.persistent = false;
                task.deps.push(Target::parse("#tag:dev").unwrap());

                TasksExpander {
                    project: &mut project,
                    workspace_root: sandbox.path(),
                }
                .expand_deps(&mut task, do_query)
                .unwrap();
            }
        }
    }

    mod expand_env {
        use super::*;

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "inner-${FOO}".into());
            task.env.insert("KEY3".into(), "$KEY1-self".into());

            env::set_var("FOO", "foo");

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
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
        fn loads_from_env_file() {
            let sandbox = create_sandbox("env-file");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_file = Some(InputPath::ProjectFile(".env".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.options.env_file = Some(InputPath::WorkspaceFile(".env-shared".into()));

            env::set_var("EXTERNAL", "external-value");

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
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
        fn skips_missing_env_file() {
            let sandbox = create_sandbox("env-file");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_file = Some(InputPath::ProjectFile(".env-missing".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.options.env_file = Some(InputPath::ProjectFile(".env-invalid".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_env(&mut task)
            .unwrap();
        }
    }

    mod expand_inputs {
        use super::*;

        #[test]
        fn extracts_env_var() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs.push(InputPath::EnvVar("FOO_BAR".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_inputs(&mut task)
            .unwrap();

            assert_eq!(task.input_vars, FxHashSet::from_iter(["FOO_BAR".into()]));
            assert_eq!(task.input_globs, FxHashSet::default());
            assert_eq!(task.input_paths, FxHashSet::default());
        }

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@files(all)".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_inputs(&mut task)
            .unwrap();

            assert_eq!(task.input_globs, FxHashSet::default());
            assert_eq!(
                task.input_paths,
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@group(all)".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_inputs(&mut task)
            .unwrap();

            assert_eq!(
                task.input_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.input_paths,
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs
                .push(InputPath::ProjectGlob("$task/**/*".into()));
            task.inputs
                .push(InputPath::WorkspaceFile("$project/index.js".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_inputs(&mut task)
            .unwrap();

            assert_eq!(
                task.input_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(task.input_paths, create_path_set(vec!["project/index.js"]));
        }
    }

    mod expand_outputs {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@files(all)".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_outputs(&mut task)
            .unwrap();

            assert_eq!(task.output_globs, FxHashSet::default());
            assert_eq!(
                task.output_paths,
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@group(all)".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_outputs(&mut task)
            .unwrap();

            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                task.output_paths,
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
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs
                .push(OutputPath::ProjectGlob("$task/**/*".into()));
            task.outputs
                .push(OutputPath::WorkspaceFile("$project/index.js".into()));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_outputs(&mut task)
            .unwrap();

            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(task.output_paths, create_path_set(vec!["project/index.js"]));
        }

        #[test]
        fn doesnt_overlap_input_file() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs.push(OutputPath::ProjectFile("out".into()));
            task.input_paths.insert(project.source.join("out"));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_outputs(&mut task)
            .unwrap();

            assert!(task.input_paths.is_empty());
            assert_eq!(
                task.output_paths,
                create_path_set(vec!["project/source/out"])
            );
        }

        #[test]
        fn doesnt_overlap_input_glob() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs
                .push(OutputPath::ProjectGlob("out/**/*".into()));
            task.input_globs.insert(project.source.join("out/**/*"));

            TasksExpander {
                project: &mut project,
                workspace_root: sandbox.path(),
            }
            .expand_outputs(&mut task)
            .unwrap();

            assert!(task.input_globs.is_empty());
            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/out/**/*"])
            );
        }
    }
}
