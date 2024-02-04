mod utils;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{DependencyConfig, InputPath, OutputPath, TaskArgs, TaskDependencyConfig};
use moon_project::Project;
use moon_project_expander::TasksExpander;
use moon_task::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::env;
use std::path::Path;
use utils::{
    create_context, create_context_with_query, create_project, create_project_with_tasks,
    create_task,
};

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
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "@dirs(group)".into();

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
                .expand_command(&mut task)
                .unwrap();
        }

        #[test]
        fn replaces_token_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.command = "./$project/bin".into();

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
                .expand_command(&mut task)
                .unwrap();

            assert_eq!(task.command, "./foo-self");
        }
    }

    mod expand_args {
        use super::*;

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_args(&mut task).unwrap();

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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.args = vec!["a".into(), "@files(all)".into(), "b".into()];
            task.options.run_from_workspace_root = true;

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_args(&mut task).unwrap();

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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_args(&mut task).unwrap();

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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_args(&mut task).unwrap();

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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_args(&mut task).unwrap();

            assert_eq!(task.args, ["a", "foo-bar-self", "b"]);
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
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse(":build").unwrap()));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }
        }

        mod deps {
            use super::*;

            #[test]
            fn no_depends_on() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
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
                    .push(DependencyConfig::new("foo".into()));

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        TaskDependencyConfig::new(Target::parse("foo:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("bar:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("baz:build").unwrap()),
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
                    .push(DependencyConfig::new("foo".into()));

                let mut task = create_task();
                task.options.persistent = false;
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("^:dev").unwrap()));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Task project:task cannot depend on task foo:test-fail, as it is allowed to"
            )]
            fn errors_for_allow_failure_chain() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                // The valid list comes from `query` but we need a
                // non-empty set for the expansion to work.
                project
                    .dependencies
                    .push(DependencyConfig::new("foo".into()));

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("^:test-fail").unwrap(),
                ));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }
        }

        mod own_self {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("~:build").unwrap()));
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("lint").unwrap()));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        TaskDependencyConfig::new(Target::parse("project:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("project:lint").unwrap()),
                    ]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("task").unwrap()));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn ignores_dupes() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("~:test").unwrap()));
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("test").unwrap()));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig::new(
                        Target::parse("project:test").unwrap()
                    )]
                );
            }

            #[test]
            #[should_panic(
                expected = "Invalid dependency project:unknown for project:task, target does not"
            )]
            fn errors_unknown_sibling_task() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("~:unknown").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.options.persistent = false;
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("~:dev").unwrap()));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }
        }

        mod project {
            use super::*;

            #[test]
            fn refs_sibling_task() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("project:build").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig::new(
                        Target::parse("project:build").unwrap()
                    )]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("project:task").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn refs_other_project_tasks() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("foo:build").unwrap(),
                ));
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("bar:lint").unwrap(),
                ));
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("baz:test").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.filtered(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        TaskDependencyConfig::new(Target::parse("foo:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("bar:lint").unwrap()),
                        TaskDependencyConfig::new(Target::parse("baz:test").unwrap()),
                    ]
                );
            }

            #[test]
            #[should_panic(
                expected = "Invalid dependency foo:unknown for project:task, target does not exist."
            )]
            fn errors_unknown_task() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("foo:unknown").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.options.persistent = false;
                task.deps
                    .push(TaskDependencyConfig::new(Target::parse("foo:dev").unwrap()));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }
        }

        mod tag {
            use super::*;

            #[test]
            fn no_tags() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("#tag:build").unwrap(),
                ));

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn returns_tasks_of_same_name() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("#tag:build").unwrap(),
                ));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        TaskDependencyConfig::new(Target::parse("foo:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("bar:build").unwrap()),
                        TaskDependencyConfig::new(Target::parse("baz:build").unwrap()),
                    ]
                );
            }

            #[test]
            fn ignores_self_ref_cycle() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let cloned_project = project.clone();

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("#tag:task").unwrap(),
                ));

                let context = create_context_with_query(&project, sandbox.path(), |_| {
                    Ok(vec![&cloned_project])
                });
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            #[should_panic(
                expected = "Non-persistent task project:task cannot depend on persistent task"
            )]
            fn errors_for_persistent_chain() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.options.persistent = false;
                task.deps.push(TaskDependencyConfig::new(
                    Target::parse("#tag:dev").unwrap(),
                ));

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();
            }
        }

        mod config {
            use super::*;

            #[test]
            fn passes_args_through() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();

                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::String("a b c".into()),
                    target: Target::parse("test").unwrap(),
                    ..TaskDependencyConfig::default()
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        target: Target::parse("project:test").unwrap(),
                        ..TaskDependencyConfig::default()
                    }]
                );
            }

            #[test]
            fn passes_env_through() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();

                task.deps.push(TaskDependencyConfig {
                    env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                    target: Target::parse("test").unwrap(),
                    ..TaskDependencyConfig::default()
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig {
                        args: TaskArgs::None,
                        env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                        target: Target::parse("project:test").unwrap(),
                        optional: None,
                    }]
                );
            }

            #[test]
            fn passes_args_and_env_through() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();

                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::String("a b c".into()),
                    env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                    target: Target::parse("test").unwrap(),
                    optional: None,
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig {
                        args: TaskArgs::String("a b c".into()),
                        env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                        target: Target::parse("project:test").unwrap(),
                        optional: None,
                    }]
                );
            }

            #[test]
            fn expands_parent_scope() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                // The valid list comes from `query` but we need a
                // non-empty set for the expansion to work.
                project
                    .dependencies
                    .push(DependencyConfig::new("foo".into()));

                let mut task = create_task();

                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::String("a b c".into()),
                    env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                    target: Target::parse("^:build").unwrap(),
                    optional: None,
                });

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![
                        TaskDependencyConfig {
                            args: TaskArgs::String("a b c".into()),
                            env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                            target: Target::parse("foo:build").unwrap(),
                            optional: None,
                        },
                        TaskDependencyConfig {
                            args: TaskArgs::String("a b c".into()),
                            env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                            target: Target::parse("bar:build").unwrap(),
                            optional: None,
                        },
                        TaskDependencyConfig {
                            args: TaskArgs::String("a b c".into()),
                            env: FxHashMap::from_iter([("FOO".into(), "bar".into())]),
                            target: Target::parse("baz:build").unwrap(),
                            optional: None,
                        }
                    ]
                );
            }

            #[test]
            fn skip_missing_self_targets() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();

                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([]),
                    target: Target::parse("do-not-exist").unwrap(),
                    optional: Some(true),
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.filtered(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(task.deps, vec![]);
            }

            #[test]
            fn resolve_self_targets_when_optional() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([]),
                    target: Target::parse("build").unwrap(),
                    optional: Some(true),
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.filtered(i));
                TasksExpander::new(&context).expand_deps(&mut task).unwrap();

                assert_eq!(
                    task.deps,
                    vec![TaskDependencyConfig {
                        args: TaskArgs::None,
                        env: FxHashMap::from_iter([]),
                        target: Target::parse("project:build").unwrap(),
                        optional: Some(true),
                    },]
                );
            }

            #[test]
            fn error_on_missing_self_targets() {
                let sandbox = create_empty_sandbox();
                let project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([]),
                    target: Target::parse("do-not-exist").unwrap(),
                    optional: Some(false),
                });

                let context =
                    create_context_with_query(&project, sandbox.path(), |i| query.none(i));
                let error = TasksExpander::new(&context)
                    .expand_deps(&mut task)
                    .unwrap_err();

                assert_eq!(
                error.to_string(),
                "Invalid dependency project:do-not-exist for project:task, target does not exist."
            );
            }

            #[test]
            fn error_on_missing_deps_target() {
                let sandbox = create_empty_sandbox();
                let mut project = create_project_with_tasks(sandbox.path(), "project");
                let query = QueryContainer::new(sandbox.path());

                // The valid list comes from `query` but we need a
                // non-empty set for the expansion to work.
                project
                    .dependencies
                    .push(DependencyConfig::new("foo".into()));

                let mut task = create_task();
                task.deps.push(TaskDependencyConfig {
                    args: TaskArgs::None,
                    env: FxHashMap::from_iter([]),
                    target: Target::parse("^:do-not-exist").unwrap(),
                    optional: Some(false),
                });

                let context = create_context_with_query(&project, sandbox.path(), |i| query.all(i));
                let error = TasksExpander::new(&context)
                    .expand_deps(&mut task)
                    .unwrap_err();

                assert_eq!(
                    error.to_string(),
                    "Invalid dependency foo:do-not-exist for project:task, target does not exist."
                );
            }
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

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "@globs(all)".into());
            task.env.insert("KEY2".into(), "$project-$task".into());

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

            assert_eq!(
                task.env,
                FxHashMap::from_iter([
                    (
                        "KEY1".into(),
                        "project/source/*.md,project/source/**/*.json".into()
                    ),
                    ("KEY2".into(), "project-task".into()),
                ])
            );
        }

        #[test]
        fn can_use_env_vars_and_token_vars() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env
                .insert("KEY".into(), "$project-$FOO-$unknown".into());

            env::set_var("FOO", "foo");

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

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
            task.options.env_file = Some(InputPath::ProjectFile(".env".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

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
            task.options.env_file = Some(InputPath::WorkspaceFile(".env-shared".into()));

            env::set_var("EXTERNAL", "external-value");

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

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
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.env.insert("KEY1".into(), "value1".into());
            task.env.insert("KEY2".into(), "value2".into());
            task.options.env_file = Some(InputPath::ProjectFile(".env-missing".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();

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
            task.options.env_file = Some(InputPath::ProjectFile(".env-invalid".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context).expand_env(&mut task).unwrap();
        }
    }

    mod expand_inputs {
        use super::*;

        #[test]
        fn extracts_env_var() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::EnvVar("FOO_BAR".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
                .expand_inputs(&mut task)
                .unwrap();

            assert_eq!(task.input_vars, FxHashSet::from_iter(["FOO_BAR".into()]));
            assert_eq!(task.input_globs, FxHashSet::default());
            assert_eq!(task.input_files, FxHashSet::default());
        }

        #[test]
        fn replaces_token_funcs() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@files(all)".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs.push(InputPath::ProjectFile("file.txt".into()));
            task.inputs.push(InputPath::TokenFunc("@group(all)".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.inputs
                .push(InputPath::ProjectGlob("$task/**/*".into()));
            task.inputs
                .push(InputPath::WorkspaceFile("$project/index.js".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@files(all)".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectFile("file.txt".into()));
            task.outputs
                .push(OutputPath::TokenFunc("@group(all)".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectGlob("$task/**/*".into()));
            task.outputs
                .push(OutputPath::WorkspaceFile("$project/index.js".into()));

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
                .expand_outputs(&mut task)
                .unwrap();

            assert_eq!(
                task.output_globs,
                create_path_set(vec!["project/source/task/**/*"])
            );
            assert_eq!(task.output_files, create_path_set(vec!["project/index.js"]));
        }

        #[test]
        fn doesnt_overlap_input_file() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs.push(OutputPath::ProjectFile("out".into()));
            task.input_files.insert("project/source/out".into());

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
                .expand_outputs(&mut task)
                .unwrap();

            assert!(task.input_files.is_empty());
            assert_eq!(
                task.output_files,
                create_path_set(vec!["project/source/out"])
            );
        }

        #[test]
        fn doesnt_overlap_input_glob() {
            let sandbox = create_sandbox("input-group");
            let project = create_project(sandbox.path());

            let mut task = create_task();
            task.outputs
                .push(OutputPath::ProjectGlob("out/**/*".into()));
            task.input_globs.insert("project/source/out/**/*".into());

            let context = create_context(&project, sandbox.path());
            TasksExpander::new(&context)
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
