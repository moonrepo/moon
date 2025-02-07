use moon_common::Id;
use moon_config::*;
use moon_task::{Target, Task, TaskOptions};
use moon_task_builder::{TaskDepsBuilder, TasksQuerent};
use rustc_hash::FxHashMap;

#[derive(Default)]
struct TestQuerent {
    pub data: FxHashMap<Target, TaskOptions>,
    pub tag_ids: Vec<Id>,
}

impl TasksQuerent for TestQuerent {
    fn query_projects_by_tag(&self, _tag: &str) -> miette::Result<Vec<&Id>> {
        Ok(self.tag_ids.iter().collect())
    }

    fn query_tasks(
        &self,
        project_ids: Vec<&Id>,
        task_id: &Id,
    ) -> miette::Result<Vec<(&Target, &TaskOptions)>> {
        Ok(self
            .data
            .iter()
            .filter_map(|(target, options)| {
                let project_id = target.get_project_id()?;

                if &target.task_id == task_id && project_ids.contains(&project_id) {
                    Some((target, options))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
}

fn create_task() -> Task {
    Task {
        id: Id::raw("task"),
        target: Target::new("project", "task").unwrap(),
        ..Task::default()
    }
}

fn build_task_deps(task: &mut Task) {
    build_task_deps_with_data(task, FxHashMap::default());
}

fn build_task_deps_with_data(task: &mut Task, data: FxHashMap<Target, TaskOptions>) {
    let project_id = Id::raw("project");
    let project_dependencies = vec![];

    TaskDepsBuilder {
        querent: Box::new(TestQuerent {
            data,
            tag_ids: vec![],
        }),
        project_id: &project_id,
        project_dependencies: &project_dependencies,
        task,
    }
    .build()
    .unwrap()
}

mod task_deps_builder {
    use super::*;

    #[test]
    #[should_panic(expected = "Task project:task cannot depend on task project:allow-failure")]
    fn errors_if_dep_on_allow_failure() {
        let mut task = create_task();
        task.deps.push(TaskDependencyConfig::new(
            Target::parse("allow-failure").unwrap(),
        ));

        build_task_deps_with_data(
            &mut task,
            FxHashMap::from_iter([(
                Target::parse("project:allow-failure").unwrap(),
                TaskOptions {
                    allow_failure: true,
                    ..Default::default()
                },
            )]),
        );
    }

    #[test]
    #[should_panic(expected = "Task project:task cannot depend on task project:no-ci")]
    fn errors_if_dep_not_run_in_ci() {
        let mut task = create_task();
        task.options.run_in_ci = TaskOptionRunInCI::Enabled(true);
        task.deps
            .push(TaskDependencyConfig::new(Target::parse("no-ci").unwrap()));

        build_task_deps_with_data(
            &mut task,
            FxHashMap::from_iter([(
                Target::parse("project:no-ci").unwrap(),
                TaskOptions {
                    run_in_ci: TaskOptionRunInCI::Enabled(false),
                    ..Default::default()
                },
            )]),
        );
    }

    #[test]
    fn doesnt_errors_if_dep_run_in_ci() {
        let mut task = create_task();
        task.options.run_in_ci = TaskOptionRunInCI::Enabled(false);
        task.deps
            .push(TaskDependencyConfig::new(Target::parse("ci").unwrap()));

        build_task_deps_with_data(
            &mut task,
            FxHashMap::from_iter([(
                Target::parse("project:ci").unwrap(),
                TaskOptions {
                    run_in_ci: TaskOptionRunInCI::Enabled(true),
                    ..Default::default()
                },
            )]),
        );
    }

    #[test]
    #[should_panic(expected = "Non-persistent task project:task cannot depend on persistent task")]
    fn errors_for_invalid_persistent_chain() {
        let mut task = create_task();
        task.options.persistent = false;
        task.deps.push(TaskDependencyConfig::new(
            Target::parse("persistent").unwrap(),
        ));

        build_task_deps_with_data(
            &mut task,
            FxHashMap::from_iter([(
                Target::parse("project:persistent").unwrap(),
                TaskOptions {
                    persistent: true,
                    ..Default::default()
                },
            )]),
        );
    }

    #[test]
    fn doesnt_errors_for_valid_persistent_chain() {
        let mut task = create_task();
        task.options.persistent = true;
        task.deps.push(TaskDependencyConfig::new(
            Target::parse("not-persistent").unwrap(),
        ));

        build_task_deps_with_data(
            &mut task,
            FxHashMap::from_iter([(
                Target::parse("project:not-persistent").unwrap(),
                TaskOptions {
                    persistent: false,
                    ..Default::default()
                },
            )]),
        );
    }

    mod all_scope {
        use super::*;

        #[test]
        #[should_panic(
            expected = "Invalid dependency :build for project:task. All (:) scope is not"
        )]
        fn errors_for_all_scope() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse(":build").unwrap()));

            build_task_deps(&mut task);
        }
    }

    mod parent_deps_scope {
        use super::*;

        #[test]
        fn no_depends_on() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()));

            build_task_deps(&mut task);

            assert!(task.deps.is_empty());
        }

        #[test]
        fn returns_each_parent_task() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()));

            let project_id = Id::raw("project");
            let project_dependencies = vec![
                DependencyConfig::new(Id::raw("foo")),
                DependencyConfig::new(Id::raw("bar")),
                DependencyConfig::new(Id::raw("baz")),
                DependencyConfig::new(Id::raw("qux")),
            ];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([
                        (Target::parse("foo:build").unwrap(), TaskOptions::default()),
                        (Target::parse("bar:build").unwrap(), TaskOptions::default()),
                        (Target::parse("baz:build").unwrap(), TaskOptions::default()),
                    ]),
                    tag_ids: vec![],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("bar:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("baz:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("foo:build").unwrap()),
                ]
            );
        }

        #[test]
        fn returns_each_parent_task_only_if_id_matches() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()));

            let project_id = Id::raw("project");
            let project_dependencies = vec![
                DependencyConfig::new(Id::raw("foo")),
                DependencyConfig::new(Id::raw("bar")),
                DependencyConfig::new(Id::raw("baz")),
            ];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([
                        (Target::parse("foo:build").unwrap(), TaskOptions::default()),
                        (Target::parse("bar:test").unwrap(), TaskOptions::default()),
                        (Target::parse("baz:lint").unwrap(), TaskOptions::default()),
                        // Ignored
                        (Target::parse("qux:build").unwrap(), TaskOptions::default()),
                    ]),
                    tag_ids: vec![],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("foo:build").unwrap()
                )]
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid dependency ^:build for project:task, no matching targets"
        )]
        fn can_error_if_non_optional_and_no_results() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("^:build").unwrap()).required());

            let project_id = Id::raw("project");
            let project_dependencies = vec![
                DependencyConfig::new(Id::raw("foo")),
                DependencyConfig::new(Id::raw("bar")),
                DependencyConfig::new(Id::raw("baz")),
            ];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::default(),
                    tag_ids: vec![],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();
        }
    }

    mod self_scope {
        use super::*;

        fn create_project_task_data() -> FxHashMap<Target, TaskOptions> {
            FxHashMap::from_iter([
                (
                    Target::parse("project:build").unwrap(),
                    TaskOptions::default(),
                ),
                (
                    Target::parse("project:lint").unwrap(),
                    TaskOptions::default(),
                ),
                (
                    Target::parse("project:test").unwrap(),
                    TaskOptions::default(),
                ),
                // Self
                (
                    Target::parse("project:task").unwrap(),
                    TaskOptions::default(),
                ),
            ])
        }

        #[test]
        fn returns_sibling_task() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("~:build").unwrap()));
            // Without scope
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("lint").unwrap()));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("project:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("project:lint").unwrap()),
                ]
            );
        }

        #[test]
        fn ignores_self_cycle() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("~:task").unwrap()));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(task.deps, vec![]);
        }

        #[test]
        fn ignores_dupe_ids() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("~:build").unwrap()));
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("build").unwrap()));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("project:build").unwrap()
                )]
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid dependency ~:unknown for project:task, target does not exist"
        )]
        fn errors_if_unknown() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("~:unknown").unwrap(),
            ));

            build_task_deps_with_data(&mut task, create_project_task_data());
        }

        #[test]
        fn doesnt_error_if_unknown_but_optional() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("~:unknown").unwrap()).optional());

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert!(task.deps.is_empty());
        }
    }

    mod id_scope {
        use super::*;

        fn create_project_task_data() -> FxHashMap<Target, TaskOptions> {
            FxHashMap::from_iter([
                (Target::parse("a:build").unwrap(), TaskOptions::default()),
                (Target::parse("b:lint").unwrap(), TaskOptions::default()),
                (Target::parse("c:test").unwrap(), TaskOptions::default()),
                // Self
                (
                    Target::parse("project:task").unwrap(),
                    TaskOptions::default(),
                ),
            ])
        }

        #[test]
        fn returns_task() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("a:build").unwrap()));
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("c:test").unwrap()));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("a:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("c:test").unwrap()),
                ]
            );
        }

        #[test]
        fn ignores_self_cycle() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("project:task").unwrap(),
            ));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(task.deps, vec![]);
        }

        #[test]
        fn ignores_dupe_ids() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("a:build").unwrap()));
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("a:build").unwrap()));

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(Target::parse("a:build").unwrap())]
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid dependency d:unknown for project:task, target does not exist"
        )]
        fn errors_if_unknown() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("d:unknown").unwrap(),
            ));

            build_task_deps_with_data(&mut task, create_project_task_data());
        }

        #[test]
        fn doesnt_error_if_unknown_but_optional() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("d:unknown").unwrap()).optional());

            build_task_deps_with_data(&mut task, create_project_task_data());

            assert!(task.deps.is_empty());
        }
    }

    mod tag_scope {
        use super::*;

        #[test]
        fn no_tags() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("#pkg:build").unwrap(),
            ));

            build_task_deps(&mut task);

            assert!(task.deps.is_empty());
        }

        #[test]
        fn returns_each_tag_task() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("#pkg:build").unwrap(),
            ));

            let project_id = Id::raw("project");
            let project_dependencies = vec![];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([
                        (Target::parse("foo:build").unwrap(), TaskOptions::default()),
                        (Target::parse("bar:build").unwrap(), TaskOptions::default()),
                        (Target::parse("baz:build").unwrap(), TaskOptions::default()),
                    ]),
                    tag_ids: vec![Id::raw("foo"), Id::raw("baz")],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();

            assert_eq!(
                task.deps,
                vec![
                    TaskDependencyConfig::new(Target::parse("baz:build").unwrap()),
                    TaskDependencyConfig::new(Target::parse("foo:build").unwrap()),
                ]
            );
        }

        #[test]
        fn returns_each_tag_task_only_if_id_matches() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("#pkg:build").unwrap(),
            ));

            let project_id = Id::raw("project");
            let project_dependencies = vec![];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([
                        (Target::parse("foo:build").unwrap(), TaskOptions::default()),
                        (Target::parse("bar:lint").unwrap(), TaskOptions::default()),
                        (Target::parse("baz:test").unwrap(), TaskOptions::default()),
                    ]),
                    tag_ids: vec![Id::raw("foo"), Id::raw("baz")],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();

            assert_eq!(
                task.deps,
                vec![TaskDependencyConfig::new(
                    Target::parse("foo:build").unwrap()
                ),]
            );
        }

        #[test]
        #[should_panic(
            expected = "Invalid dependency #pkg:build for project:task, no matching targets"
        )]
        fn can_error_if_non_optional_and_no_results() {
            let mut task = create_task();
            task.deps
                .push(TaskDependencyConfig::new(Target::parse("#pkg:build").unwrap()).required());

            let project_id = Id::raw("project");
            let project_dependencies = vec![];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([]),
                    tag_ids: vec![Id::raw("foo"), Id::raw("baz")],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();
        }

        #[test]
        fn ignores_self_cycle() {
            let mut task = create_task();
            task.deps.push(TaskDependencyConfig::new(
                Target::parse("#pkg:task").unwrap(),
            ));

            let project_id = Id::raw("project");
            let project_dependencies = vec![];

            TaskDepsBuilder {
                querent: Box::new(TestQuerent {
                    data: FxHashMap::from_iter([(
                        Target::parse("project:task").unwrap(),
                        TaskOptions::default(),
                    )]),
                    tag_ids: vec![Id::raw("project")],
                }),
                project_id: &project_id,
                project_dependencies: &project_dependencies,
                task: &mut task,
            }
            .build()
            .unwrap();

            assert!(task.deps.is_empty());
        }
    }
}
