use moon_config::{
    GlobalProjectConfig, PlatformType, ProjectConfig, ProjectDependsOn, ProjectLanguage,
    ProjectMetadataConfig, ProjectType, TargetID, TaskCommandArgs, TaskConfig, TaskMergeStrategy,
    TaskOptionsConfig,
};
use moon_project::{Project, ProjectError};
use moon_task::{EnvVars, FileGroup, Target, Task};
use moon_test_utils::{get_fixtures_path, get_fixtures_root};
use moon_utils::string_vec;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::Path;

fn mock_file_groups() -> FxHashMap<String, FileGroup> {
    FxHashMap::from_iter([(
        String::from("sources"),
        FileGroup::new("sources", string_vec!["src/**/*"]),
    )])
}

fn mock_global_project_config() -> GlobalProjectConfig {
    GlobalProjectConfig {
        extends: None,
        file_groups: FxHashMap::from_iter([(String::from("sources"), string_vec!["src/**/*"])]),
        tasks: BTreeMap::new(),
        schema: String::new(),
    }
}

fn create_expanded_project(
    id: &str,
    source: &str,
    workspace_root: &Path,
    config: &GlobalProjectConfig,
) -> Project {
    let mut project = Project::new(id, source, workspace_root, config).unwrap();
    project.expand_tasks(workspace_root, &[], &[]).unwrap();
    project
}

#[test]
#[should_panic(expected = "MissingProjectAtSource(\"projects/missing\")")]
fn doesnt_exist() {
    Project::new(
        "missing",
        "projects/missing",
        &get_fixtures_root(),
        &mock_global_project_config(),
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
                file_groups: FxHashMap::from_iter([(
                    String::from("tests"),
                    string_vec!["**/*_test.rs"]
                )]),
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
            file_groups: FxHashMap::from_iter([(String::from("tests"), string_vec!["tests/**/*"])]),
            ..GlobalProjectConfig::default()
        },
    )
    .unwrap();

    assert_eq!(
        project,
        Project {
            id: String::from("basic"),
            config: ProjectConfig {
                depends_on: vec![ProjectDependsOn::String("noConfig".to_owned())],
                file_groups: FxHashMap::from_iter([(
                    String::from("tests"),
                    string_vec!["**/*_test.rs"]
                )]),
                language: ProjectLanguage::JavaScript,
                ..ProjectConfig::default()
            },
            log_target: String::from("moon:project:basic"),
            root: workspace_root.join("projects/basic"),
            file_groups: FxHashMap::from_iter([(
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
    use moon_test_utils::pretty_assertions::assert_eq;
    use moon_utils::glob;

    fn mock_task_config(command: &str) -> TaskConfig {
        TaskConfig {
            command: Some(TaskCommandArgs::String(command.to_owned())),
            ..TaskConfig::default()
        }
    }

    fn mock_merged_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_env: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            retry_count: Some(1),
            run_deps_in_parallel: Some(true),
            run_in_ci: Some(true),
            ..TaskOptionsConfig::default()
        }
    }

    fn mock_local_task_options_config(strategy: TaskMergeStrategy) -> TaskOptionsConfig {
        TaskOptionsConfig {
            merge_args: Some(strategy.clone()),
            merge_deps: Some(strategy.clone()),
            merge_env: Some(strategy.clone()),
            merge_inputs: Some(strategy.clone()),
            merge_outputs: Some(strategy),
            ..TaskOptionsConfig::default()
        }
    }

    fn stub_global_task_options_config() -> TaskOptionsConfig {
        TaskOptionsConfig {
            cache: Some(true),
            retry_count: Some(1),
            run_deps_in_parallel: Some(true),
            run_in_ci: Some(true),
            ..TaskOptionsConfig::default()
        }
    }

    fn stub_global_env_vars() -> EnvVars {
        FxHashMap::from_iter([
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

        let mut parts = target.split(':');
        parts.next();

        task.log_target = format!("moon:project:{}", target);
        task.id = parts.next().unwrap().to_string();
        task.target = Target::parse(&target).unwrap();

        Ok(task)
    }

    #[test]
    fn inherits_global_tasks() {
        let workspace_root = get_fixtures_root();
        let project = create_expanded_project(
            "id",
            "tasks/no-tasks",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
        );

        let mut task = Task::from_config(
            Target::new("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        )
        .unwrap();
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
        let project = create_expanded_project(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
        );

        let mut build = Task::from_config(
            Target::new("id", "build").unwrap(),
            &mock_task_config("webpack"),
        )
        .unwrap();
        build.platform = PlatformType::Node;

        let mut std = Task::from_config(
            Target::new("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        )
        .unwrap();
        std.platform = PlatformType::System;

        let mut test = Task::from_config(
            Target::new("id", "test").unwrap(),
            &mock_task_config("jest"),
        )
        .unwrap();
        test.platform = PlatformType::Node;

        let mut lint = Task::from_config(
            Target::new("id", "lint").unwrap(),
            &mock_task_config("eslint"),
        )
        .unwrap();
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
    fn inherits_implicit_deps() {
        let workspace_root = get_fixtures_root();
        let mut project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
        )
        .unwrap();

        project
            .expand_tasks(
                &workspace_root,
                &string_vec!["~:build", "project:task"],
                &[],
            )
            .unwrap();

        let mut build = Task::from_config(
            Target::new("id", "build").unwrap(),
            &mock_task_config("webpack"),
        )
        .unwrap();

        let mut std = Task::from_config(
            Target::new("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        )
        .unwrap();

        let mut test = Task::from_config(
            Target::new("id", "test").unwrap(),
            &mock_task_config("jest"),
        )
        .unwrap();

        let mut lint = Task::from_config(
            Target::new("id", "lint").unwrap(),
            &mock_task_config("eslint"),
        )
        .unwrap();

        // Expanded
        let implicit_deps = string_vec!["id:build", "project:task"];

        build
            .deps
            .extend(Task::create_dep_targets(&implicit_deps).unwrap());
        std.deps
            .extend(Task::create_dep_targets(&implicit_deps).unwrap());
        test.deps
            .extend(Task::create_dep_targets(&implicit_deps).unwrap());
        lint.deps
            .extend(Task::create_dep_targets(&implicit_deps).unwrap());

        assert_eq!(project.get_task("build").unwrap().deps, build.deps);
        assert_eq!(project.get_task("standard").unwrap().deps, std.deps);
        assert_eq!(project.get_task("test").unwrap().deps, test.deps);
        assert_eq!(project.get_task("lint").unwrap().deps, lint.deps);
    }

    #[test]
    fn inherits_implicit_deps_self_target() {
        let workspace_root = get_fixtures_root();
        let mut project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
        )
        .unwrap();

        project
            .expand_tasks(&workspace_root, &string_vec!["build"], &[])
            .unwrap();

        let mut build = Task::from_config(
            Target::new("id", "build").unwrap(),
            &mock_task_config("webpack"),
        )
        .unwrap();

        // Expanded
        build
            .deps
            .extend(Task::create_dep_targets(&string_vec!["id:build"]).unwrap());

        assert_eq!(project.get_task("build").unwrap().deps, build.deps);
    }

    #[test]
    fn inherits_implicit_inputs() {
        let workspace_root = get_fixtures_root();
        let implicit_inputs = string_vec!["$VAR", "package.json", "/.moon/workspace.yml"];
        let mut project = Project::new(
            "id",
            "tasks/basic",
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(String::from("standard"), mock_task_config("cmd"))]),
                ..GlobalProjectConfig::default()
            },
        )
        .unwrap();

        project
            .expand_tasks(&workspace_root, &[], &implicit_inputs)
            .unwrap();

        let mut build = Task::from_config(
            Target::new("id", "build").unwrap(),
            &mock_task_config("webpack"),
        )
        .unwrap();

        let mut std = Task::from_config(
            Target::new("id", "standard").unwrap(),
            &mock_task_config("cmd"),
        )
        .unwrap();

        let mut test = Task::from_config(
            Target::new("id", "test").unwrap(),
            &mock_task_config("jest"),
        )
        .unwrap();

        let mut lint = Task::from_config(
            Target::new("id", "lint").unwrap(),
            &mock_task_config("eslint"),
        )
        .unwrap();

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
            FxHashSet::from_iter(["VAR".to_owned()])
        );
    }

    #[test]
    fn strategy_replace() {
        let workspace_root = get_fixtures_root();
        let project_source = "tasks/merge-replace";
        let project = create_expanded_project(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(TaskCommandArgs::Sequence(string_vec!["--a"])),
                        command: Some(TaskCommandArgs::String("standard".to_owned())),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        local: false,
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        platform: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
        );

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
                            command: Some(TaskCommandArgs::String("newcmd".to_owned())),
                            deps: Some(string_vec!["buildB:standard"]),
                            env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                            local: false,
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Replace),
                            platform: PlatformType::System,
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
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
                            command: Some(TaskCommandArgs::String("newcmd".to_owned())),
                            deps: Some(string_vec!["buildB:standard"]),
                            env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                            local: false,
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Replace),
                            platform: PlatformType::System,
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
        let project = create_expanded_project(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(TaskCommandArgs::Sequence(string_vec!["--a"])),
                        command: Some(TaskCommandArgs::String("standard".to_owned())),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        local: false,
                        inputs: Some(string_vec!["a.*"]),
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        platform: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
        );

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
                            command: None,
                            deps: Some(string_vec!["buildB:standard"]),
                            env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                            local: false,
                            inputs: Some(string_vec!["b.*"]),
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Append),
                            platform: PlatformType::System,
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
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--a", "--b"])),
                            command: Some(TaskCommandArgs::String("standard".to_owned())),
                            deps: Some(string_vec!["a:standard", "buildB:standard"]),
                            env: Some(FxHashMap::from_iter([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "b".to_owned())
                            ])),
                            inputs: Some(string_vec!["a.*", "b.*"]),
                            local: false,
                            outputs: Some(string_vec!["a.ts", "b.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Append),
                            platform: PlatformType::System,
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
        let project = create_expanded_project(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(TaskCommandArgs::Sequence(string_vec!["--a"])),
                        command: Some(TaskCommandArgs::String("standard".to_owned())),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        local: false,
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        platform: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
        );

        assert_eq!(
            project,
            Project {
                id: String::from("id"),
                config: ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("standard"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
                            command: Some(TaskCommandArgs::String("newcmd".to_owned())),
                            deps: Some(string_vec!["buildB:standard"]),
                            env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            local: false,
                            outputs: Some(string_vec!["b.ts"]),
                            options: mock_local_task_options_config(TaskMergeStrategy::Prepend),
                            platform: PlatformType::System,
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
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b", "--a"])),
                            command: Some(TaskCommandArgs::String("newcmd".to_owned())),
                            deps: Some(string_vec!["buildB:standard", "a:standard"]),
                            env: Some(FxHashMap::from_iter([
                                ("GLOBAL".to_owned(), "1".to_owned()),
                                ("KEY".to_owned(), "a".to_owned())
                            ])),
                            inputs: Some(string_vec!["b.*", "a.*"]),
                            local: false,
                            outputs: Some(string_vec!["b.ts", "a.ts"]),
                            options: mock_merged_task_options_config(TaskMergeStrategy::Prepend),
                            platform: PlatformType::System,
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
        let project = create_expanded_project(
            "id",
            project_source,
            &workspace_root,
            &GlobalProjectConfig {
                tasks: BTreeMap::from([(
                    String::from("standard"),
                    TaskConfig {
                        args: Some(TaskCommandArgs::Sequence(string_vec!["--a"])),
                        command: Some(TaskCommandArgs::String("standard".to_owned())),
                        deps: Some(string_vec!["a:standard"]),
                        env: Some(stub_global_env_vars()),
                        inputs: Some(string_vec!["a.*"]),
                        local: false,
                        outputs: Some(string_vec!["a.ts"]),
                        options: stub_global_task_options_config(),
                        platform: PlatformType::Node,
                    },
                )]),
                ..GlobalProjectConfig::default()
            },
        );

        let mut task = create_expanded_task(
            Target::format("id", "standard").unwrap(),
            TaskConfig {
                args: Some(TaskCommandArgs::Sequence(string_vec!["--a", "--b"])),
                command: Some(TaskCommandArgs::String("standard".to_owned())),
                deps: Some(string_vec!["buildB:standard", "a:standard"]),
                env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                inputs: Some(string_vec!["b.*"]),
                local: false,
                outputs: Some(string_vec!["a.ts", "b.ts"]),
                options: TaskOptionsConfig {
                    cache: Some(true),
                    merge_args: Some(TaskMergeStrategy::Append),
                    merge_deps: Some(TaskMergeStrategy::Prepend),
                    merge_env: Some(TaskMergeStrategy::Replace),
                    merge_inputs: Some(TaskMergeStrategy::Replace),
                    merge_outputs: Some(TaskMergeStrategy::Append),
                    retry_count: Some(1),
                    run_deps_in_parallel: Some(true),
                    run_in_ci: Some(true),
                    run_from_workspace_root: None,
                    ..TaskOptionsConfig::default()
                },
                platform: PlatformType::Unknown,
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
                            args: Some(TaskCommandArgs::Sequence(string_vec!["--b"])),
                            command: None,
                            deps: Some(string_vec!["buildB:standard"]),
                            env: Some(FxHashMap::from_iter([("KEY".to_owned(), "b".to_owned())])),
                            inputs: Some(string_vec!["b.*"]),
                            local: false,
                            outputs: Some(string_vec!["b.ts"]),
                            options: TaskOptionsConfig {
                                merge_args: Some(TaskMergeStrategy::Append),
                                merge_deps: Some(TaskMergeStrategy::Prepend),
                                merge_env: Some(TaskMergeStrategy::Replace),
                                merge_inputs: Some(TaskMergeStrategy::Replace),
                                merge_outputs: Some(TaskMergeStrategy::Append),
                                ..TaskOptionsConfig::default()
                            },
                            platform: PlatformType::Unknown,
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
        use moon_test_utils::pretty_assertions::assert_eq;

        #[test]
        fn resolves_self_scope() {
            let project = create_expanded_project(
                "id",
                "self",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                Task::create_dep_targets(&string_vec!["id:clean", "id:build"]).unwrap()
            );
        }

        #[test]
        fn resolves_self_scope_without_prefix() {
            let project = create_expanded_project(
                "id",
                "self-no-prefix",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                Task::create_dep_targets(&string_vec!["id:clean", "id:build"]).unwrap()
            );
        }

        #[test]
        fn resolves_self_scope_without_dupes() {
            let project = create_expanded_project(
                "id",
                "self-dupes",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );

            assert_eq!(
                project.tasks.get("lint").unwrap().deps,
                Task::create_dep_targets(&string_vec!["id:build"]).unwrap()
            );
        }

        #[test]
        fn resolves_deps_scope() {
            let project = create_expanded_project(
                "id",
                "deps",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );

            assert_eq!(
                project.tasks.get("build").unwrap().deps,
                Task::create_dep_targets(&string_vec!["bar:build", "baz:build", "foo:build"])
                    .unwrap()
            );
        }

        #[test]
        fn resolves_deps_scope_without_dupes() {
            let project = create_expanded_project(
                "id",
                "deps-dupes",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );

            assert_eq!(
                project.tasks.get("build").unwrap().deps,
                Task::create_dep_targets(&string_vec!["foo:build", "bar:build", "baz:build"])
                    .unwrap()
            );
        }

        #[test]
        #[should_panic(expected = "Target(NoProjectAllInTaskDeps(\":build\"))")]
        fn errors_for_all_scope() {
            create_expanded_project(
                "id",
                "all",
                &get_fixtures_path("task-deps"),
                &mock_global_project_config(),
            );
        }
    }

    mod tokens {
        use super::*;
        use moon_config::DependencyConfig;
        use moon_project::ProjectDependency;
        use moon_test_utils::pretty_assertions::assert_eq;
        use std::path::PathBuf;

        #[test]
        fn expands_args() {
            let project = create_expanded_project(
                "id",
                "base/files-and-dirs",
                &get_fixtures_root(),
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ])),
                            command: Some(TaskCommandArgs::String("test".to_owned())),
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
            );

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
            let project = create_expanded_project(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec![
                                "--dirs",
                                "@dirs(static)",
                                "--files",
                                "@files(static)",
                                "--globs",
                                "@globs(globs)",
                                "--root",
                                "@root(static)",
                            ])),
                            command: Some(TaskCommandArgs::String("test".to_owned())),
                            options: TaskOptionsConfig {
                                run_from_workspace_root: Some(true),
                                ..TaskOptionsConfig::default()
                            },
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
            );

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
            let project = create_expanded_project(
                "id",
                "base/files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            args: Some(TaskCommandArgs::Sequence(string_vec![
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
                            ])),
                            command: Some(TaskCommandArgs::String("test".to_owned())),
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
            );

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
            let workspace_root = get_fixtures_path("base");
            let project_root = workspace_root.join("files-and-dirs");
            let project = create_expanded_project(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("test".to_owned())),
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
            );

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([
                    glob::normalize(project_root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project_root.join("*.js")).unwrap()
                ]),
            );

            let a: FxHashSet<PathBuf> =
                FxHashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: FxHashSet<PathBuf> = FxHashSet::from_iter(
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
        fn expands_implicit_deps() {
            let workspace_root = get_fixtures_path("base");
            let mut project = Project::new(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("test".to_owned())),
                            deps: Some(string_vec!["~:test"]),
                            platform: PlatformType::Node,
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
            )
            .unwrap();

            project.dependencies.insert(
                "example".into(),
                ProjectDependency::from_config(&DependencyConfig::new("example")),
            );

            project
                .expand_tasks(
                    &workspace_root,
                    &string_vec!["^:build", "project:task"],
                    &[],
                )
                .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.deps,
                Task::create_dep_targets(&string_vec!["id:test", "example:build", "project:task"])
                    .unwrap()
            );
        }

        #[test]
        fn expands_implicit_inputs() {
            let workspace_root = get_fixtures_path("base");
            let project_root = workspace_root.join("files-and-dirs");
            let mut project = Project::new(
                "id",
                "files-and-dirs",
                &workspace_root,
                &GlobalProjectConfig {
                    file_groups: create_file_groups_config(),
                    tasks: BTreeMap::from([(
                        String::from("test"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("test".to_owned())),
                            inputs: Some(string_vec!["local.ts",]),
                            platform: PlatformType::Node,
                            ..TaskConfig::default()
                        },
                    )]),
                    ..GlobalProjectConfig::default()
                },
            )
            .unwrap();

            project
                .expand_tasks(
                    &workspace_root,
                    &[],
                    &[
                        "/.moon/$taskPlatform-$projectType.yml".to_owned(),
                        "*.yml".to_owned(),
                    ],
                )
                .unwrap();

            let task = project.tasks.get("test").unwrap();

            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([glob::normalize(project_root.join("*.yml")).unwrap()])
            );

            let a: FxHashSet<PathBuf> =
                FxHashSet::from_iter(task.input_paths.iter().map(PathBuf::from));
            let b: FxHashSet<PathBuf> = FxHashSet::from_iter(
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
                file_groups: FxHashMap::default(),
                tasks: BTreeMap::from([
                    (
                        String::from("a"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("a".to_owned())),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("b"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("b".to_owned())),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        String::from("c"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("c".to_owned())),
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
            let project = create_expanded_project(
                "id",
                "include",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec!["a", "c"])
        }

        #[test]
        fn include_none() {
            let project = create_expanded_project(
                "id",
                "include-none",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude() {
            let project = create_expanded_project(
                "id",
                "exclude",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec!["b"])
        }

        #[test]
        fn exclude_all() {
            let project = create_expanded_project(
                "id",
                "exclude-all",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec![])
        }

        #[test]
        fn exclude_none() {
            let project = create_expanded_project(
                "id",
                "exclude-none",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec!["a", "b", "c"])
        }

        #[test]
        fn rename() {
            let project = create_expanded_project(
                "id",
                "rename",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(
                get_project_task_ids(project),
                string_vec!["bar", "baz", "foo"]
            )
        }

        #[test]
        fn rename_merge() {
            let workspace_root = get_fixtures_path("task-inheritance");
            let project = create_expanded_project(
                "id",
                "rename-merge",
                &workspace_root,
                &mock_global_project_config(),
            );

            let mut task =
                create_expanded_task(&workspace_root, &workspace_root.join("rename-merge"), None)
                    .unwrap();
            task.id = "foo".to_owned();
            task.target = Target::new("id", "foo").unwrap();
            task.command = "a".to_owned();
            task.args.push("renamed-and-merge-foo".to_owned());
            task.log_target = "moon:project:id:foo".to_owned();

            assert_eq!(*project.get_task("foo").unwrap(), task);

            assert_eq!(get_project_task_ids(project), string_vec!["b", "c", "foo"]);
        }

        #[test]
        fn include_exclude() {
            let project = create_expanded_project(
                "id",
                "include-exclude",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec!["a"])
        }

        #[test]
        fn include_exclude_rename() {
            let project = create_expanded_project(
                "id",
                "include-exclude-rename",
                &get_fixtures_path("task-inheritance"),
                &mock_global_project_config(),
            );

            assert_eq!(get_project_task_ids(project), string_vec!["only"])
        }
    }
}
