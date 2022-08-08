use moon_config::{
    GlobalProjectConfig, PlatformType, ProjectConfig, ProjectDependsOn, ProjectLanguage,
    ProjectMetadataConfig, ProjectType, TargetID, TaskConfig, TaskMergeStrategy, TaskOptionsConfig,
};
use moon_project::{Project, ProjectError};
use moon_task::{EnvVars, FileGroup, Target, Task};
use moon_utils::string_vec;
use moon_utils::test::{get_fixtures_dir, get_fixtures_root};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

fn mock_file_groups() -> HashMap<String, FileGroup> {
    HashMap::from([(
        String::from("sources"),
        FileGroup::new("sources", string_vec!["src/**/*"]),
    )])
}

fn mock_global_project_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        extends: None,
        file_groups: HashMap::from([(String::from("sources"), string_vec!["src/**/*"])]),
        tasks: BTreeMap::new(),
        schema: String::new(),
    }
}

#[test]
#[should_panic(expected = "MissingProject(\"projects/missing\")")]
fn doesnt_exist() {
    Project::new(
        "missing",
        "projects/missing",
        &get_fixtures_root(),
        &mock_global_project_config(),
        &[],
    )
    .unwrap();
}

#[test]
fn no_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "no-config",
        "projects/no-config",
        &workspace_root,
        &mock_global_project_config(),
        &[],
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("no-config"),
            log_target: String::from("moon:project:no-config"),
            root: workspace_root.join("projects/no-config"),
            file_groups: mock_file_groups(),
            source: String::from("projects/no-config"),
            ..Project::default()
        }
    );
}

#[test]
fn empty_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "empty-config",
        "projects/empty-config",
        &workspace_root,
        &mock_global_project_config(),
        &[],
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("empty-config"),
            config: ProjectConfig::default(),
            log_target: String::from("moon:project:empty-config"),
            root: workspace_root.join("projects/empty-config"),
            file_groups: mock_file_groups(),
            source: String::from("projects/empty-config"),
            ..Project::default()
        }
    );
}

#[test]
fn basic_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &workspace_root,
        &mock_global_project_config(),
        &[],
    )
    .unwrap();
    let project_root = workspace_root.join("projects/basic");

    // Merges with global
    let mut file_groups = mock_file_groups();
    file_groups.insert(
        String::from("tests"),
        FileGroup::new("tests", string_vec!["**/*_test.rs"]),
    );

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: ProjectConfig {
                depends_on: vec![ProjectDependsOn::String("noConfig".to_owned())],
                file_groups: HashMap::from([(String::from("tests"), string_vec!["**/*_test.rs"])]),
                language: ProjectLanguage::JavaScript,
                ..ProjectConfig::default()
            },
            log_target: String::from("moon:project:basic"),
            root: project_root,
            file_groups,
            source: String::from("projects/basic"),
            ..Project::default()
        }
    );
}

#[test]
fn advanced_config() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "advanced",
        "projects/advanced",
        &workspace_root,
        &mock_global_project_config(),
        &[],
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("advanced"),
            config: ProjectConfig {
                project: Some(ProjectMetadataConfig {
                    name: String::from("Advanced"),
                    description: String::from("Advanced example."),
                    owner: String::from("Batman"),
                    maintainers: string_vec!["Bruce Wayne"],
                    channel: String::from("#batcave"),
                }),
                type_of: ProjectType::Application,
                language: ProjectLanguage::TypeScript,
                ..ProjectConfig::default()
            },
            log_target: String::from("moon:project:advanced"),
            root: workspace_root.join("projects/advanced"),
            file_groups: mock_file_groups(),
            source: String::from("projects/advanced"),
            ..Project::default()
        }
    );
}

#[test]
fn overrides_global_file_groups() {
    let workspace_root = get_fixtures_root();
    let project = Project::new(
        "basic",
        "projects/basic",
        &workspace_root,
        &GlobalProjectConfig {
            file_groups: HashMap::from([(String::from("tests"), string_vec!["tests/**/*"])]),
            ..GlobalProjectConfig::default()
        },
        &[],
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: ProjectConfig {
                depends_on: vec![ProjectDependsOn::String("noConfig".to_owned())],
                file_groups: HashMap::from([(String::from("tests"), string_vec!["**/*_test.rs"])]),
                language: ProjectLanguage::JavaScript,
                ..ProjectConfig::default()
            },
            log_target: String::from("moon:project:basic"),
            root: workspace_root.join("projects/basic"),
            file_groups: HashMap::from([(
                String::from("tests"),
                FileGroup::new("tests", string_vec!["**/*_test.rs"],)
            )]),
            source: String::from("projects/basic"),
            ..Project::default()
        }
    );
}

mod tasks {
    use super::*;
    use moon_task::test::{
        create_expanded_task as create_expanded_task_internal, create_file_groups_config,
    };
    use moon_utils::glob;
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;

    fn mock_task_config(command: &str) -> TaskConfig {
        TaskConfig {
            command: Some(command.to_owned()),
            ..TaskConfig::default()
        }
    }

    fn mock_merged_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            cache: None,
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_env: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            output_style: None,
            retry_count: Some(1),
            run_deps_in_parallel: Some(true),
            run_in_ci: Some(true),
            run_from_workspace_root: None,
        }
    }

    fn mock_local_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            cache: None,
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_env: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            output_style: None,
            retry_count: None,
            run_deps_in_parallel: None,
            run_in_ci: None,
            run_from_workspace_root: None,
        }
    }

    fn stub_global_task_options_config() -> TaskOptionsConfig {
        TaskOptionsConfig {
            cache: Some(true),
            merge_args: None,
            merge_deps: None,
            merge_env: None,
            merge_inputs: None,
            merge_outputs: None,
            output_style: None,
            retry_count: Some(1),
            run_deps_in_parallel: Some(true),
            run_in_ci: Some(true),
            run_from_workspace_root: None,
        }
    }

    fn stub_global_env_vars() -> EnvVars {
        HashMap::from([
            ("GLOBAL".to_owned(), "1".to_owned()),
            ("KEY".to_owned(), "a".to_owned()),
        ])
    }

    fn create_expanded_task(
        target: TargetID,
        config: TaskConfig,
        workspace_root: &Path,
        project_source: &str,
    ) -> Result<Task, ProjectError> {
        let project_root = workspace_root.join(project_source);
        let mut task =
            create_expanded_task_internal(workspace_root, &project_root, Some(config)).unwrap();

        task.log_target = format!("moon:project:{}", target);
        task.target = target;

        Ok(task)
    }

    #[test]
    fn inherits_global_tasks() {
        let workspace_root = get_fixtures_root();
        let project = Project::new(
            "id",
            "tasks/no-tasks",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        let mut task = Task::from_config(
            Target::format("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        );
        task.platform = PlatformType::System;

        // Expanded
        task.input_globs
            .insert(glob::normalize(&workspace_root.join("tasks/no-tasks/**/*")).unwrap());

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    language: ProjectLanguage::JavaScript,
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join("tasks/no-tasks"),
                source: String::from("tasks/no-tasks"),
                tasks: BTreeMap::from([(String::from("standard"), task)]),
                ..Project::default()
            }
        );
    }

    #[test]
    fn merges_with_global_tasks() {
        let workspace_root = get_fixtures_root();
        let project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        let mut build = Task::from_config(
            Target::format("id", "build").unwrap(),
            &mock_task_config("webpack"),
        );
        build.platform = PlatformType::Node;

        let mut std = Task::from_config(
            Target::format("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        );
        std.platform = PlatformType::System;

        let mut test = Task::from_config(
            Target::format("id", "test").unwrap(),
            &mock_task_config("jest"),
        );
        test.platform = PlatformType::Node;

        let mut lint = Task::from_config(
            Target::format("id", "lint").unwrap(),
            &mock_task_config("eslint"),
        );
        lint.platform = PlatformType::Node;

        // Expanded
        let wild_glob = workspace_root.join("tasks/basic/**/*");

        build
            .input_globs
            .insert(glob::normalize(&wild_glob).unwrap());
        std.input_globs.insert(glob::normalize(&wild_glob).unwrap());
        test.input_globs
            .insert(glob::normalize(&wild_glob).unwrap());
        lint.input_globs
            .insert(glob::normalize(&wild_glob).unwrap());

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    language: ProjectLanguage::JavaScript,
                    tasks: BTreeMap::from([
                        (String::from("build"), mock_task_config("webpack")),
                        (String::from("test"), mock_task_config("jest")),
                        (String::from("lint"), mock_task_config("eslint"))
                    ]),
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join("tasks/basic"),
                source: String::from("tasks/basic"),
                tasks: BTreeMap::from([
                    (String::from("build"), build),
                    (String::from("standard"), std),
                    (String::from("test"), test),
                    (String::from("lint"), lint)
                ]),
                ..Project::default()
            }
        );
    }

    #[test]
    fn inherits_implicit_inputs() {
        let workspace_root = get_fixtures_root();
        let implicit_inputs = string_vec!["$VAR", "package.json", "/.moon/workspace.yml"];
        let project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
            &implicit_inputs,
        )
        .unwrap();

        let mut build = Task::from_config(
            Target::format("id", "build").unwrap(),
            &mock_task_config("webpack"),
        );

        let mut std = Task::from_config(
            Target::format("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        );

        let mut test = Task::from_config(
            Target::format("id", "test").unwrap(),
            &mock_task_config("jest"),
        );

        let mut lint = Task::from_config(
            Target::format("id", "lint").unwrap(),
            &mock_task_config("eslint"),
        );

        // Expanded
        build.inputs.extend(implicit_inputs.clone());
        std.inputs.extend(implicit_inputs.clone());
        test.inputs.extend(implicit_inputs.clone());
        lint.inputs.extend(implicit_inputs);

        assert_eq!(project.get_task("build").unwrap().inputs, build.inputs);
        assert_eq!(project.get_task("standard").unwrap().inputs, std.inputs);
        assert_eq!(project.get_task("test").unwrap().inputs, test.inputs);
        assert_eq!(project.get_task("lint").unwrap().inputs, lint.inputs);

        // Applies to all tasks
        assert_eq!(
            project.get_task("build").unwrap().input_vars,
            HashSet::from(["VAR".to_owned()])
        );
    }

    #[test]
    fn strategy_replace() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-replace";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Replace),
                            type_of: PlatformType::System,
                        }
                    )]),
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join(project_source),
                source: String::from(project_source),
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Replace),
                            type_of: PlatformType::System,
                        },
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
                ..Project::default()
            }
        );
    }

    #[test]
    fn strategy_append() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-append";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: None,
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Append),
                            type_of: PlatformType::System,
                        }
                    )]),
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join(project_source),
                source: String::from(project_source),
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--a", "--b"]),
                            command: Some(String::from("standard")),
                            deps: Some(string_vec!["a:standard", "b:standard"]),
                            env: Some(HashMap::from([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "b".to_owned())
                            ])),
                            inputs: Some(string_vec!["a.*", "b.*"]),
                            outputs: Some(string_vec!["a.ts", "b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Append),
                            type_of: PlatformType::System,
                        },
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
                ..Project::default()
            }
        );
    }

    #[test]
    fn strategy_prepend() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-prepend";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Prepend),
                            type_of: PlatformType::System,
                        }
                    )]),
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join(project_source),
                source: String::from(project_source),
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    create_expanded_task(
                        Target::format("id", "standard").unwrap(),
                        TaskConfig {
                            args: Some(string_vec!["--b", "--a"]),
                            command: Some(String::from("newcmd")),
                            deps: Some(string_vec!["b:standard", "a:standard"]),
                            env: Some(HashMap::from([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "a".to_owned())
                            ])),
                            inputs: Some(string_vec!["b.*", "a.*"]),
                            outputs: Some(string_vec!["b.ts", "a.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Prepend),
                            type_of: PlatformType::System,
                        },
                        &workspace_root,
                        project_source
                    )
                    .unwrap()
                )]),
                ..Project::default()
            }
        );
    }

    #[test]
    fn strategy_all() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-all-strategies";
        let project = Project::new(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(string_vec!["--a"]),
                        command: Some(String::from("standard")),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        type_of: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
            &[],
        )
        .unwrap();

        let mut task = create_expanded_task(
            Target::format("id", "standard").unwrap(),
            TaskConfig {
                args: Some(string_vec!["--a", "--b"]),
                command: Some(String::from("standard")),
                deps: Some(string_vec!["b:standard", "a:standard"]),
                env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                inputs: Some(string_vec!["b.*"]),
                outputs: Some(string_vec!["a.ts", "b.ts"]),
                options: TaskOptionsConfig {
                    cache: Some(true),
                    merge_args: Some(TaskMergeStrategy::Append),
                    merge_deps: Some(TaskMergeStrategy::Prepend),
                    merge_env: Some(TaskMergeStrategy::Replace),
                    merge_inputs: Some(TaskMergeStrategy::Replace),
                    merge_outputs: Some(TaskMergeStrategy::Append),
                    output_style: None,
                    retry_count: Some(1),
                    run_deps_in_parallel: Some(true),
                    run_in_ci: Some(true),
                    run_from_workspace_root: None,
                },
                type_of: PlatformType::Unknown,
            },
            &workspace_root,
            project_source,
        )
        .unwrap();
        task.platform = PlatformType::Unknown;

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(string_vec!["--b"]),
                            command: None,
                            deps: Some(string_vec!["b:standard"]),
                            env: Some(HashMap::from([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: TaskOptionsConfig {
                                cache: None,
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_env: Some(TaskMergeStrategy::Replace),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                output_style: None,
                                retry_count: None,
                                run_deps_in_parallel: None,
                                run_in_ci: None,
                                run_from_workspace_root: None,
                            },
                            type_of: PlatformType::Unknown,
                        }
                    )]),
                    ..ProjectConfig::default()
                },
                log_target: String::from("moon:project:id"),
                root: workspace_root.join(project_source),
                source: String::from(project_source),
                tasks: BTreeMap::from([(String::from("standard"), task)]),
                ..Project::default()
            }
        );
    }

    mod expands_deps {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn resolves_self_scope() {
            let project = Project::new(
                "id",
                "self",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                string_vec!["id:clean", "id:build"]
            );
        }

        #[test]
        fn resolves_self_scope_without_dupes() {
            let project = Project::new(
                "id",
                "self-dupes",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                string_vec!["id:build"]
            );
        }

        #[test]
        fn resolves_deps_scope() {
            let project = Project::new(
                "id",
                "deps",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("build").unwrap().deps,
                string_vec!["foo:build", "bar:build", "baz:build"]
            );
        }

        #[test]
        fn resolves_deps_scope_without_dupes() {
            let project = Project::new(
                "id",
                "deps-dupes",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(
                project.tasks.get("build").unwrap().deps,
                string_vec!["foo:build", "bar:build", "baz:build"]
            );
        }

        #[test]
        #[should_panic(expected = "Target(NoProjectAllInTaskDeps(\":build\"))")]
        fn errors_for_all_scope() {
            Project::new(
                "id",
                "all",
                &get_fixtures_dir("task-deps"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();
        }
    }

    mod tokens {
        use super::*;
        use pretty_assertions::assert_eq;
        use std::collections::HashSet;
        use std::path::PathBuf;

        #[test]
        fn expands_args() {
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &get_fixtures_root(),
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ]),
                            command: Some(String::from("test")),
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
                &[],
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                if cfg!(windows) {
                    vec![
                        "--dirs",
                        ".\\dir",
                        ".\\dir\\subdir",
                        "--files",
                        ".\\file.ts",
                        ".\\dir\\other.tsx",
                        ".\\dir\\subdir\\another.ts",
                        "--globs",
                        "./**/*.{ts,tsx}",
                        "./*.js",
                        "--root",
                        ".\\dir",
                    ]
                } else {
                    vec![
                        "--dirs",
                        "./dir",
                        "./dir/subdir",
                        "--files",
                        "./file.ts",
                        "./dir/other.tsx",
                        "./dir/subdir/another.ts",
                        "--globs",
                        "./**/*.{ts,tsx}",
                        "./*.js",
                        "--root",
                        "./dir",
                    ]
                },
            )
        }

        #[test]
        fn expands_args_from_workspace() {
            let workspace_root = get_fixtures_root();
            let project_root = workspace_root.join("base").join("files-and-dirs");
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ]),
                            command: Some(String::from("test")),
                            options: TaskOptionsConfig {
                                run_from_workspace_root: Some(true),
                                ..TaskOptionsConfig::default()
                            },
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
                &[],
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                vec![
                    "--dirs",
                    project_root.join("dir").to_str().unwrap(),
                    project_root.join("dir").join("subdir").to_str().unwrap(),
                    "--files",
                    project_root.join("file.ts").to_str().unwrap(),
                    project_root.join("dir").join("other.tsx").to_str().unwrap(),
                    project_root
                        .join("dir")
                        .join("subdir")
                        .join("another.ts")
                        .to_str()
                        .unwrap(),
                    "--globs",
                    glob::remove_drive_prefix(
                        glob::normalize(project_root.join("**/*.{ts,tsx}")).unwrap()
                    )
                    .as_str(),
                    glob::remove_drive_prefix(glob::normalize(project_root.join("*.js")).unwrap())
                        .as_str(),
                    "--root",
                    project_root.join("dir").to_str().unwrap(),
                ],
            )
        }

        #[test]
        fn expands_args_with_vars() {
            let workspace_root = get_fixtures_root();
            let project_root = workspace_root.join("base").join("files-and-dirs");
            let project = Project::new(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(string_vec![
                                "some/$unknown/var", // Unknown
                                "--pid",
                                "$project/foo", // At start
                                "--proot",
                                "$projectRoot", // Alone
                                "--psource",
                                "foo/$projectSource", // At end
                                "--target",
                                "foo/$target/bar", // In middle
                                "--tid=$task",     // As an arg
                                "--wsroot",
                                "$workspaceRoot" // Alone
                            ]),
                            command: Some(String::from("test")),
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
                &[],
            )
            .unwrap();

            assert_eq!(
                *project.tasks.get("test").unwrap().args,
                vec![
                    "some/$unknown/var",
                    "--pid",
                    "id/foo",
                    "--proot",
                    project_root.to_str().unwrap(),
                    "--psource",
                    // This is wonky but also still valid
                    if cfg!(windows) {
                        "foo/base\\files-and-dirs"
                    } else {
                        "foo/base/files-and-dirs"
                    },
                    "--target",
                    "foo/id:test/bar",
                    "--tid=test",
                    "--wsroot",
                    workspace_root.to_str().unwrap(),
                ],
            )
        }

        #[test]
        fn expands_inputs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let project = Project::new(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(String::from("test")),
                            inputs: Some(string_vec![
                                "file.ts",
                                "@dirs(static)",
                                "@files(static)",
                                "@globs(globs)",
                                "@root(static)",
                                "/package.json",
                            ]),
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
                &[],
            )
            .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                HashSet::from([
                    glob::normalize(project_root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project_root.join("*.js")).unwrap()
                ]),
            );

            let a: HashSet<PathBuf> =
                HashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: HashSet<PathBuf> = HashSet::from_iter(
                vec![
                    project_root.join("file.ts"),
                    project_root.join("dir"),
                    project_root.join("dir/subdir"),
                    project_root.join("file.ts"),
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    workspace_root.join("package.json"),
                ]
                .iter()
                .map(PathBuf::from),
            );

            assert_eq!(a, b);
        }

        #[test]
        fn expands_implicit_inputs() {
            let workspace_root = get_fixtures_dir("base");
            let project_root = workspace_root.join("files-and-dirs");
            let project = Project::new(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(String::from("test")),
                            inputs: Some(string_vec!["local.ts",]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
                &[
                    "/.moon/$taskType-$projectType.yml".to_owned(),
                    "*.yml".to_owned(),
                ],
            )
            .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                HashSet::from([glob::normalize(project_root.join("*.yml")).unwrap()])
            );

            let a: HashSet<PathBuf> =
                HashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: HashSet<PathBuf> = HashSet::from_iter(
                vec![
                    project_root.join("local.ts"),
                    workspace_root.join(".moon/node-unknown.yml"),
                ]
                .iter()
                .map(PathBuf::from),
            );

            assert_eq!(a, b);
        }
    }
}

mod workspace {
    use super::*;
    use moon_task::test::create_expanded_task;

    mod inherited_tasks {
        use super::*;

        fn mock_global_project_config() -> GlobalProjectConfig {
            GlobalProjectConfig {
                file_groups: HashMap::new(),
                tasks: BTreeMap::from([
                    (
                        String::from("a"),
                        TaskConfig {
                            command: Some(String::from("a")),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("b"),
                        TaskConfig {
                            command: Some(String::from("b")),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("c"),
                        TaskConfig {
                            command: Some(String::from("c")),
                            ..TaskConfig::default()
                        },
                    ),
                ]),
                ..GlobalProjectConfig::default()
            }
        }

        fn get_project_task_ids(project: Project) -> Vec<String> {
            let mut ids = project.tasks.into_keys().collect::<Vec<String>>();
            ids.sort();
            ids
        }

        #[test]
        fn include() {
            let project = Project::new(
                "id",
                "include",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a", "c"])
        }

        #[test]
        fn include_none() {
            let project = Project::new(
                "id",
                "include-none",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude() {
            let project = Project::new(
                "id",
                "exclude",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["b"])
        }

        #[test]
        fn exclude_all() {
            let project = Project::new(
                "id",
                "exclude-all",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude_none() {
            let project = Project::new(
                "id",
                "exclude-none",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a", "b", "c"])
        }

        #[test]
        fn rename() {
            let project = Project::new(
                "id",
                "rename",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(
                get_project_task_ids(project),
                string_vec!["bar", "baz", "foo"]
            )
        }

        #[test]
        fn rename_merge() {
            let workspace_root = get_fixtures_dir("task-inheritance");
            let project = Project::new(
                "id",
                "rename-merge",
                &workspace_root,
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            let mut task =
                create_expanded_task(&workspace_root, &workspace_root.join("rename-merge"), None)
                    .unwrap();
            task.target = "id:foo".to_owned();
            task.command = "a".to_owned();
            task.args.push("renamed-and-merge-foo".to_owned());
            task.log_target = String::from("moon:project:id:foo");

            assert_eq!(*project.get_task("foo").unwrap(), task);

            assert_eq!(get_project_task_ids(project), string_vec!["b", "c", "foo"]);
        }

        #[test]
        fn include_exclude() {
            let project = Project::new(
                "id",
                "include-exclude",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["a"])
        }

        #[test]
        fn include_exclude_rename() {
            let project = Project::new(
                "id",
                "include-exclude-rename",
                &get_fixtures_dir("task-inheritance"),
                &mock_global_project_config(),
                &[],
            )
            .unwrap();

            assert_eq!(get_project_task_ids(project), string_vec!["only"])
        }
    }
}
