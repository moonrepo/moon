mod utils;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{DependencyConfig, InputPath, OutputPath};
use moon_project::Project;
use moon_task::{Target, Task};
use moon_task_expander::TasksExpander;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::env;
use std::path::Path;
use utils::{create_project, create_project_with_tasks};

fn create_path_set(inputs: Vec<&str>) -> FxHashSet<WorkspaceRelativePathBuf> {
    FxHashSet::from_iter(inputs.into_iter().map(|s| s.into()))
}

fn create_expander<'l>(
    workspace_root: &'l Path,
    project: &'l mut Project,
    func: impl FnOnce(&mut Task),
) -> TasksExpander<'l> {
    func(project.tasks.get_mut("task").unwrap());

    TasksExpander {
        project,
        workspace_root,
    }
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.command = "@dirs(group)".into();
            })
            .expand_command("task")
            .unwrap();
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.command = "./$project/bin".into();
            })
            .expand_command("task")
            .unwrap();

            assert_eq!(project.get_task("task").unwrap().command, "./project/bin");
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            env::set_var("FOO", "foo");
            env::set_var("BAZ_QUX", "baz-qux");

            create_expander(sandbox.path(), &mut project, |task| {
                task.command = "./$FOO/${BAR}/$BAZ_QUX".into();
            })
            .expand_command("task")
            .unwrap();

            env::remove_var("FOO");
            env::remove_var("BAZ_QUX");

            assert_eq!(
                project.get_task("task").unwrap().command,
                "./foo/${BAR}/baz-qux"
            );
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.command = "./$FOO".into();
                task.env.insert("FOO".into(), "foo-self".into());
            })
            .expand_command("task")
            .unwrap();

            assert_eq!(project.get_task("task").unwrap().command, "./foo-self");
        }
    }

    mod expand_args {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.args = vec!["a".into(), "@files(all)".into(), "b".into()];
            })
            .expand_args("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().args,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.args = vec!["a".into(), "@files(all)".into(), "b".into()];
                task.options.run_from_workspace_root = true;
            })
            .expand_args("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().args,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.args = vec![
                    "a".into(),
                    "$project/dir".into(),
                    "b".into(),
                    "some/$task".into(),
                ];
            })
            .expand_args("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().args,
                ["a", "project/dir", "b", "some/task"]
            );
        }

        #[test]
        fn replaces_env_vars() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            env::set_var("FOO_BAR", "foo-bar");
            env::set_var("BAR_BAZ", "bar-baz");

            create_expander(sandbox.path(), &mut project, |task| {
                task.args = vec![
                    "a".into(),
                    "$FOO_BAR".into(),
                    "b".into(),
                    "c/${BAR_BAZ}/d".into(),
                ];
            })
            .expand_args("task")
            .unwrap();

            env::remove_var("FOO_BAR");
            env::remove_var("BAR_BAZ");

            assert_eq!(
                project.get_task("task").unwrap().args,
                ["a", "foo-bar", "b", "c/bar-baz/d"]
            );
        }

        #[test]
        fn replaces_env_var_from_self() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.args = vec!["a".into(), "${FOO_BAR}".into(), "b".into()];
                task.env.insert("FOO_BAR".into(), "foo-bar-self".into());
            })
            .expand_args("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().args,
                ["a", "foo-bar-self", "b"]
            );
        }
    }

    mod expand_deps {
        use super::*;

        struct QueryContainer {
            projects: Vec<Project>,
        }

        impl QueryContainer {
            pub fn new(root: &Path) -> Self {
                Self {
                    projects: vec![
                        create_project_with_tasks(root, "foo"),
                        create_project_with_tasks(root, "bar"),
                        create_project_with_tasks(root, "baz"),
                    ],
                }
            }

            pub fn all(&self, _: String) -> miette::Result<Vec<&Project>> {
                Ok(vec![
                    &self.projects[0],
                    &self.projects[1],
                    &self.projects[2],
                ])
            }

            pub fn filtered(&self, input: String) -> miette::Result<Vec<&Project>> {
                Ok(vec![if input.contains("foo") {
                    &self.projects[0]
                } else if input.contains("bar") {
                    &self.projects[1]
                } else {
                    &self.projects[2]
                }])
            }

            pub fn none(&self, _: String) -> miette::Result<Vec<&Project>> {
                Ok(vec![])
            }
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
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse(":build").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();
            }
        }

        mod deps {
            use super::*;

            #[test]
            fn no_depends_on() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("^:build").unwrap());
                })
                .expand_deps("task", |i| query.none(i)) // Tested here
                .unwrap();

                assert_eq!(project.get_task("task").unwrap().deps, vec![]);
            }

            #[test]
            fn returns_tasks_of_same_name() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                // The valid list comes from `query` but we need a
                // non-empty set for the expansion to work.
                project
                    .dependencies
                    .insert("foo".into(), DependencyConfig::default());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("^:build").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
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
                let query = QueryContainer::new(sandbox.path());

                // The valid list comes from `query` but we need a
                // non-empty set for the expansion to work.
                project
                    .dependencies
                    .insert("foo".into(), DependencyConfig::default());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.options.persistent = false;
                    task.deps.push(Target::parse("^:dev").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();
            }
        }

        mod own_self {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("~:build").unwrap());
                    task.deps.push(Target::parse("lint").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
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
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("task").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();

                assert_eq!(project.get_task("task").unwrap().deps, vec![]);
            }

            #[test]
            fn ignores_dupes() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("~:test").unwrap());
                    task.deps.push(Target::parse("test").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
                    vec![Target::parse("project:test").unwrap()]
                );
            }

            #[test]
            #[should_panic(
                expected = "Invalid dependency project:unknown for project:task, target does not"
            )]
            fn errors_unknown_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("~:unknown").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.options.persistent = false;
                    task.deps.push(Target::parse("~:dev").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();
            }
        }

        mod project {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("project:build").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
                    vec![Target::parse("project:build").unwrap()]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("project:task").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();

                assert_eq!(project.get_task("task").unwrap().deps, vec![]);
            }

            #[test]
            fn refs_other_project_tasks() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("foo:build").unwrap());
                    task.deps.push(Target::parse("bar:lint").unwrap());
                    task.deps.push(Target::parse("baz:test").unwrap());
                })
                .expand_deps("task", |i| query.filtered(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
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
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("foo:unknown").unwrap());
                })
                .expand_deps("task", |i| query.none(i))
                .unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.options.persistent = false;
                    task.deps.push(Target::parse("foo:dev").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();
            }
        }

        mod tag {
            use super::*;

            #[test]
            fn no_tags() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("#tag:build").unwrap());
                })
                .expand_deps("task", |i| query.none(i)) // Tested here
                .unwrap();

                assert_eq!(project.get_task("task").unwrap().deps, vec![]);
            }

            #[test]
            fn returns_tasks_of_same_name() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("#tag:build").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
                .unwrap();

                assert_eq!(
                    project.get_task("task").unwrap().deps,
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

                let cloned_project = project.clone();

                create_expander(sandbox.path(), &mut project, |task| {
                    task.deps.push(Target::parse("#tag:task").unwrap());
                })
                .expand_deps("task", |_| Ok(vec![&cloned_project]))
                .unwrap();

                assert_eq!(project.get_task("task").unwrap().deps, vec![]);
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                create_expander(sandbox.path(), &mut project, |task| {
                    task.options.persistent = false;
                    task.deps.push(Target::parse("#tag:dev").unwrap());
                })
                .expand_deps("task", |i| query.all(i))
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

            env::set_var("FOO", "foo");

            create_expander(sandbox.path(), &mut project, |task| {
                task.env.insert("KEY1".into(), "value1".into());
                task.env.insert("KEY2".into(), "inner-${FOO}".into());
                task.env.insert("KEY3".into(), "$KEY1-self".into());
            })
            .expand_env("task")
            .unwrap();

            env::remove_var("FOO");

            assert_eq!(
                project.get_task("task").unwrap().env,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.env.insert("KEY1".into(), "value1".into());
                task.env.insert("KEY2".into(), "value2".into());
                task.options.env_file = Some(InputPath::ProjectFile(".env".into()));
            })
            .expand_env("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().env,
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

            env::set_var("EXTERNAL", "external-value");

            create_expander(sandbox.path(), &mut project, |task| {
                task.options.env_file = Some(InputPath::WorkspaceFile(".env-shared".into()));
            })
            .expand_env("task")
            .unwrap();

            env::remove_var("EXTERNAL");

            assert_eq!(
                project.get_task("task").unwrap().env,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.env.insert("KEY1".into(), "value1".into());
                task.env.insert("KEY2".into(), "value2".into());
                task.options.env_file = Some(InputPath::ProjectFile(".env-missing".into()));
            })
            .expand_env("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().env,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.options.env_file = Some(InputPath::ProjectFile(".env-invalid".into()));
            })
            .expand_env("task")
            .unwrap();
        }
    }

    mod expand_inputs {
        use super::*;

        #[test]
        fn extracts_env_var() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.inputs.push(InputPath::EnvVar("FOO_BAR".into()));
            })
            .expand_inputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().input_vars,
                FxHashSet::from_iter(["FOO_BAR".into()])
            );
            assert_eq!(
                project.get_task("task").unwrap().input_globs,
                FxHashSet::default()
            );
            assert_eq!(
                project.get_task("task").unwrap().input_paths,
                FxHashSet::default()
            );
        }

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.inputs.push(InputPath::ProjectFile("file.txt".into()));
                task.inputs.push(InputPath::TokenFunc("@files(all)".into()));
            })
            .expand_inputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().input_globs,
                FxHashSet::default()
            );
            assert_eq!(
                project.get_task("task").unwrap().input_paths,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.inputs.push(InputPath::ProjectFile("file.txt".into()));
                task.inputs.push(InputPath::TokenFunc("@group(all)".into()));
            })
            .expand_inputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().input_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                project.get_task("task").unwrap().input_paths,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.inputs
                    .push(InputPath::ProjectGlob("$task/**/*".into()));
                task.inputs
                    .push(InputPath::WorkspaceFile("$project/index.js".into()));
            })
            .expand_inputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().input_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(
                project.get_task("task").unwrap().input_paths,
                create_path_set(vec!["project/index.js"])
            );
        }
    }

    mod expand_outputs {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.outputs
                    .push(OutputPath::ProjectFile("file.txt".into()));
                task.outputs
                    .push(OutputPath::TokenFunc("@files(all)".into()));
            })
            .expand_outputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().output_globs,
                FxHashSet::default()
            );
            assert_eq!(
                project.get_task("task").unwrap().output_paths,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.outputs
                    .push(OutputPath::ProjectFile("file.txt".into()));
                task.outputs
                    .push(OutputPath::TokenFunc("@group(all)".into()));
            })
            .expand_outputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().output_globs,
                create_path_set(vec!["project/source/*.md", "project/source/**/*.json"])
            );
            assert_eq!(
                project.get_task("task").unwrap().output_paths,
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

            create_expander(sandbox.path(), &mut project, |task| {
                task.outputs
                    .push(OutputPath::ProjectGlob("$task/**/*".into()));
                task.outputs
                    .push(OutputPath::WorkspaceFile("$project/index.js".into()));
            })
            .expand_outputs("task")
            .unwrap();

            assert_eq!(
                project.get_task("task").unwrap().output_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(
                project.get_task("task").unwrap().output_paths,
                create_path_set(vec!["project/index.js"])
            );
        }

        #[test]
        fn doesnt_overlap_input_file() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.outputs.push(OutputPath::ProjectFile("out".into()));
                task.input_paths.insert("project/source/out".into());
            })
            .expand_outputs("task")
            .unwrap();

            assert!(project.get_task("task").unwrap().input_paths.is_empty());
            assert_eq!(
                project.get_task("task").unwrap().output_paths,
                create_path_set(vec!["project/source/out"])
            );
        }

        #[test]
        fn doesnt_overlap_input_glob() {
            let sandbox = create_sandbox("file-group");
            let mut project = create_project(sandbox.path());

            create_expander(sandbox.path(), &mut project, |task| {
                task.outputs
                    .push(OutputPath::ProjectGlob("out/**/*".into()));
                task.input_globs.insert("project/source/out/**/*".into());
            })
            .expand_outputs("task")
            .unwrap();

            assert!(project.get_task("task").unwrap().input_globs.is_empty());
            assert_eq!(
                project.get_task("task").unwrap().output_globs,
                create_path_set(vec!["project/source/out/**/*"])
            );
        }
    }
}
