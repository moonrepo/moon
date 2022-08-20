use moon_config::{TaskCommandArgs, TaskConfig};
use moon_lang_node::package::PackageJson;
use moon_platform_node::task::{create_task, should_run_in_ci, TaskContext};
use moon_platform_node::{create_tasks_from_scripts, infer_tasks_from_scripts};
use moon_task::{PlatformType, Task, TaskOptions};
use moon_utils::string_vec;
use std::collections::{BTreeMap, HashMap};

mod should_run_in_ci {
    use super::*;

    #[test]
    fn handles_reserved_words() {
        assert!(!should_run_in_ci("dev", ""));
        assert!(!should_run_in_ci("serve", ""));
        assert!(!should_run_in_ci("start", ""));

        assert!(should_run_in_ci("dev:app", ""));
        assert!(!should_run_in_ci("serve:app", ""));
        assert!(!should_run_in_ci("start:app", ""));

        assert!(should_run_in_ci("app:dev", ""));
        assert!(!should_run_in_ci("app:serve", ""));
        assert!(!should_run_in_ci("app:start", ""));
    }

    #[test]
    fn handles_watch_mode() {
        assert!(!should_run_in_ci("name", "packemon build --watch"));
        assert!(!should_run_in_ci("name", "rollup --watch"));
        assert!(!should_run_in_ci("name", "tsc --watch"));
    }

    #[test]
    fn handles_gatsby() {
        // yes
        assert!(should_run_in_ci("name", "gatsby --version"));
        assert!(should_run_in_ci("name", "gatsby --help"));
        assert!(should_run_in_ci("name", "gatsby build"));
        assert!(should_run_in_ci("name", "gatsby info"));
        assert!(should_run_in_ci("name", "npx gatsby build"));

        // no
        assert!(!should_run_in_ci("name", "gatsby dev"));
        assert!(!should_run_in_ci("name", "gatsby develop"));
        assert!(!should_run_in_ci("name", "gatsby new"));
        assert!(!should_run_in_ci("name", "gatsby serve"));
        assert!(!should_run_in_ci("name", "gatsby repl"));
    }

    #[test]
    fn handles_nextjs() {
        // yes
        assert!(should_run_in_ci("name", "next --version"));
        assert!(should_run_in_ci("name", "next --help"));
        assert!(should_run_in_ci("name", "next build"));
        assert!(should_run_in_ci("name", "next export"));
        assert!(should_run_in_ci("name", "npx next build"));

        // no
        assert!(!should_run_in_ci("name", "next dev"));
        assert!(!should_run_in_ci("name", "next start"));
    }

    #[test]
    fn handles_parcel() {
        // yes
        assert!(should_run_in_ci("name", "parcel --version"));
        assert!(should_run_in_ci("name", "parcel --help"));
        assert!(should_run_in_ci("name", "parcel build"));
        assert!(should_run_in_ci("name", "npx parcel build"));

        // no
        assert!(!should_run_in_ci("name", "parcel ./src/index.ts"));
        assert!(!should_run_in_ci("name", "parcel serve index.js"));
        assert!(!should_run_in_ci("name", "parcel watch"));
        assert!(!should_run_in_ci("name", "npx parcel"));
    }

    #[test]
    fn handles_react_scripts() {
        // yes
        assert!(should_run_in_ci("name", "react-scripts --version"));
        assert!(should_run_in_ci("name", "react-scripts --help"));
        assert!(should_run_in_ci("name", "react-scripts build"));
        assert!(should_run_in_ci("name", "react-scripts eject"));
        assert!(should_run_in_ci("name", "npx react-scripts build"));

        // no
        assert!(!should_run_in_ci("name", "react-scripts start"));
        assert!(!should_run_in_ci("name", "react-scripts test --watch"));
    }

    #[test]
    fn handles_snowpack() {
        // yes
        assert!(should_run_in_ci("name", "snowpack --version"));
        assert!(should_run_in_ci("name", "snowpack --help"));
        assert!(should_run_in_ci("name", "snowpack build"));
        assert!(should_run_in_ci("name", "npx snowpack build"));

        // no
        assert!(!should_run_in_ci("name", "snowpack dev"));
    }

    #[test]
    fn handles_vite() {
        // yes
        assert!(should_run_in_ci("name", "vite --version"));
        assert!(should_run_in_ci("name", "vite --help"));
        assert!(should_run_in_ci("name", "vite build"));
        assert!(should_run_in_ci("name", "vite optimize"));
        assert!(should_run_in_ci("name", "npx vite build"));

        // no
        assert!(!should_run_in_ci("name", "vite --watch"));
        assert!(!should_run_in_ci("name", "vite"));
        assert!(!should_run_in_ci("name", "vite dev"));
        assert!(!should_run_in_ci("name", "vite serve"));
        assert!(!should_run_in_ci("name", "vite preview"));
        assert!(!should_run_in_ci("name", "npx vite"));
        assert!(!should_run_in_ci("name", "npx vite dev"));
    }

    #[test]
    fn handles_webpack() {
        // yes
        assert!(should_run_in_ci("name", "webpack --version"));
        assert!(should_run_in_ci("name", "webpack --help"));
        assert!(should_run_in_ci("name", "webpack build"));
        assert!(should_run_in_ci("name", "webpack bundle"));
        assert!(should_run_in_ci("name", "webpack info"));
        assert!(should_run_in_ci("name", "npx webpack build"));

        // no
        assert!(!should_run_in_ci("name", "webpack --entry"));
        assert!(!should_run_in_ci("name", "webpack --watch"));
        assert!(!should_run_in_ci("name", "webpack"));
        assert!(!should_run_in_ci("name", "webpack s"));
        assert!(!should_run_in_ci("name", "webpack serve"));
        assert!(!should_run_in_ci("name", "webpack server"));
        assert!(!should_run_in_ci("name", "webpack w"));
        assert!(!should_run_in_ci("name", "webpack watch"));
        assert!(!should_run_in_ci("name", "npx webpack serve"));
    }
}

mod create_task {
    use super::*;

    mod script_files {
        use super::*;

        #[test]
        fn handles_bash() {
            let task = create_task(
                "project:task",
                "script",
                "bash scripts/setup.sh",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec![
                        "bash",
                        "scripts/setup.sh"
                    ])),
                    type_of: PlatformType::System,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn handles_bash_without_command() {
            let task = create_task(
                "project:task",
                "script",
                "scripts/setup.sh",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec![
                        "bash",
                        "scripts/setup.sh"
                    ])),
                    type_of: PlatformType::System,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn handles_node() {
            let task = create_task(
                "project:task",
                "script",
                "node scripts/test.js",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec![
                        "node",
                        "scripts/test.js"
                    ])),
                    type_of: PlatformType::Node,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn handles_node_without_command() {
            let candidates = ["scripts/test.js", "scripts/test.cjs", "scripts/test.mjs"];

            for candidate in candidates {
                let task = create_task(
                    "project:task",
                    "script",
                    candidate,
                    TaskContext::ConvertToTask,
                )
                .unwrap();

                assert_eq!(
                    task,
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec!["node", candidate])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                )
            }
        }
    }

    mod env_vars {
        use super::*;

        #[test]
        fn extracts_single_var() {
            let task = create_task(
                "project:task",
                "script",
                "KEY=VALUE yarn install",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec!["yarn", "install"])),
                    env: Some(HashMap::from([("KEY".to_owned(), "VALUE".to_owned())])),
                    type_of: PlatformType::Node,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn extracts_multiple_vars() {
            let task = create_task(
                "project:task",
                "script",
                "KEY1=VAL1 KEY2=VAL2 yarn install",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec!["yarn", "install"])),
                    env: Some(HashMap::from([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ])),
                    type_of: PlatformType::Node,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn handles_semicolons() {
            let task = create_task(
                "project:task",
                "script",
                "KEY1=VAL1; KEY2=VAL2; yarn install",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::Sequence(string_vec!["yarn", "install"])),
                    env: Some(HashMap::from([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ])),
                    type_of: PlatformType::Node,
                    ..TaskConfig::default()
                }
            )
        }

        #[test]
        fn handles_quoted_values() {
            let task = create_task(
                "project:task",
                "script",
                "NODE_OPTIONS='-f -b' yarn",
                TaskContext::ConvertToTask,
            )
            .unwrap();

            assert_eq!(
                task,
                TaskConfig {
                    command: Some(TaskCommandArgs::String("yarn".to_owned())),
                    env: Some(HashMap::from([(
                        "NODE_OPTIONS".to_owned(),
                        "-f -b".to_owned()
                    )])),
                    type_of: PlatformType::Node,
                    ..TaskConfig::default()
                }
            )
        }
    }

    mod outputs {
        use super::*;

        #[test]
        fn detects_outputs_from_args() {
            let candidates = vec![
                ("-o", "dir", "dir"),
                ("-o", "./file.js", "file.js"),
                ("--out", "./lib", "lib"),
                ("--out-dir", "build", "build"),
                ("--out-file", "./build/min.js", "build/min.js"),
                ("--outdir", "build", "build"),
                ("--outfile", "./build/min.js", "build/min.js"),
                ("--outDir", "build", "build"),
                ("--outFile", "./build/min.js", "build/min.js"),
                ("--dist", "dist", "dist"),
                ("--dist-dir", "./dist", "dist"),
                ("--dist-file", "./dist/bundle.js", "dist/bundle.js"),
                ("--distDir", "dist", "dist"),
                ("--distFile", "dist/bundle.js", "dist/bundle.js"),
            ];

            for candidate in candidates {
                let task = create_task(
                    "project:task",
                    "script",
                    &format!("tool build {} {}", candidate.0, candidate.1),
                    TaskContext::ConvertToTask,
                )
                .unwrap();

                assert_eq!(
                    task,
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "tool",
                            "build",
                            candidate.0,
                            candidate.1
                        ])),
                        outputs: Some(string_vec![candidate.2]),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                )
            }
        }

        #[should_panic(expected = "NoParentOutput(\"../parent/dir\", \"project:task\")")]
        #[test]
        fn fails_on_parent_relative() {
            create_task(
                "project:task",
                "script",
                "build --out ../parent/dir",
                TaskContext::ConvertToTask,
            )
            .unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"/abs/dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute() {
            create_task(
                "project:task",
                "script",
                "build --out /abs/dir",
                TaskContext::ConvertToTask,
            )
            .unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"C:\\\\abs\\\\dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute_windows() {
            create_task(
                "project:task",
                "script",
                "build --out C:\\\\abs\\\\dir",
                TaskContext::ConvertToTask,
            )
            .unwrap();
        }
    }
}

mod infer_tasks_from_scripts {
    use super::*;

    #[test]
    fn wraps_scripts() {
        let pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("postinstall".into(), "./setup.sh".into()),
                ("build:app".into(), "webpack build --output ./dist".into()),
                ("dev".into(), "webpack dev".into()),
                ("test".into(), "jest .".into()),
                ("posttest".into(), "run-coverage".into()),
                ("lint".into(), "eslint src/**/* .".into()),
                ("typecheck".into(), "tsc --build".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = infer_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "build-app".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "moon",
                            "node",
                            "run-script",
                            "build:app"
                        ])),
                        outputs: Some(string_vec!["dist"]),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "dev".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "moon",
                            "node",
                            "run-script",
                            "dev"
                        ])),
                        local: true,
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "test".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "moon",
                            "node",
                            "run-script",
                            "test"
                        ])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "lint".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "moon",
                            "node",
                            "run-script",
                            "lint"
                        ])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "typecheck".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "moon",
                            "node",
                            "run-script",
                            "typecheck"
                        ])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
            ])
        )
    }
}

mod create_tasks_from_scripts {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn ignores_unsupported_syntax() {
        let mut pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("cd".into(), "cd website && yarn build".into()),
                ("out".into(), "some-bin > output.log".into()),
                ("in".into(), "output.log < some-bin".into()),
                ("pipe".into(), "ls | grep foo".into()),
                ("or".into(), "foo || bar".into()),
                ("semi".into(), "foo ;; bar".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

        assert!(tasks.is_empty());
    }

    #[test]
    fn renames_to_ids() {
        let mut pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("base".into(), "script".into()),
                ("foo-bar".into(), "script".into()),
                ("foo_bar".into(), "script".into()),
                ("foo:bar".into(), "script".into()),
                ("foo-bar:baz".into(), "script".into()),
                ("foo_bar:baz".into(), "script".into()),
                ("foo:bar:baz".into(), "script".into()),
                ("foo_bar:baz-qux".into(), "script".into()),
                ("fooBar".into(), "script".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

        assert_eq!(
            tasks.keys().cloned().collect::<Vec<String>>(),
            string_vec![
                "base",
                "foo-bar",
                "foo-bar-baz",
                "fooBar",
                "foo_bar",
                "foo_bar-baz",
                "foo_bar-baz-qux",
            ]
        );
    }

    #[test]
    fn converts_stand_alone() {
        let mut pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("test".into(), "jest .".into()),
                ("lint".into(), "eslint src/**/* .".into()),
                ("typecheck".into(), "tsc --build".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

        assert_eq!(pkg.scripts, None);

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "test".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec!["jest", "."])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "lint".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec![
                            "eslint", "src/**/*", "."
                        ])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
                (
                    "typecheck".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec!["tsc", "--build"])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                ),
            ])
        )
    }

    mod pre_post {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn creates_pre_and_post() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("pretest".into(), "do something".into()),
                    ("posttest".into(), "do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "pretest".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "do",
                                "something"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "posttest".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["do", "another"])),
                            deps: Some(string_vec!["~:test"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["jest", "."])),
                            deps: Some(string_vec!["~:pretest"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            )
        }

        #[test]
        fn supports_multiple_pre_via_andand() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("pretest".into(), "do something && do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "pretest-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "do",
                                "something"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "pretest".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["do", "another"])),
                            deps: Some(string_vec!["~:pretest-dep1"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["jest", "."])),
                            deps: Some(string_vec!["~:pretest"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    )
                ])
            )
        }

        #[test]
        fn supports_multiple_post_via_andand() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("posttest".into(), "do something && do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "posttest-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "do",
                                "something"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "posttest".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["do", "another"])),
                            deps: Some(string_vec!["~:posttest-dep1", "~:test"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["jest", "."])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            )
        }

        #[test]
        fn handles_pre_within_script() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("release".into(), "npm run prerelease && npm publish".into()),
                    ("prerelease".into(), "webpack build".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "prerelease".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "webpack", "build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "release".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["npm", "publish"])),
                            deps: Some(string_vec!["~:prerelease"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            )
        }
    }

    mod pm_run {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn skips_when_pointing_to_an_unknown() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("lint".into(), "eslint .".into()),
                    ("lint:fix".into(), "npm run invalid -- --fix".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([(
                    "lint".to_owned(),
                    TaskConfig {
                        command: Some(TaskCommandArgs::Sequence(string_vec!["eslint", "."])),
                        type_of: PlatformType::Node,
                        ..TaskConfig::default()
                    }
                )])
            )
        }

        #[test]
        fn converts_without_args() {
            let candidates = [
                "npm run lint",
                "npm run lint --",
                "pnpm run lint",
                "pnpm run lint --",
                "yarn run lint",
                "yarn run lint --",
            ];

            for candidate in candidates {
                let mut pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("lint".into(), "eslint .".into()),
                        ("lint:fix".into(), candidate.to_owned()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

                assert_eq!(pkg.scripts, None);

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".to_owned(),
                            TaskConfig {
                                command: Some(TaskCommandArgs::Sequence(string_vec![
                                    "eslint", "."
                                ])),
                                type_of: PlatformType::Node,
                                ..TaskConfig::default()
                            }
                        ),
                        (
                            "lint-fix".to_owned(),
                            TaskConfig {
                                command: Some(TaskCommandArgs::Sequence(string_vec![
                                    "moon",
                                    "run",
                                    "project:lint"
                                ])),
                                type_of: PlatformType::Node,
                                ..TaskConfig::default()
                            }
                        ),
                    ])
                )
            }
        }

        #[test]
        fn converts_with_args() {
            let candidates = [
                "npm run lint -- --fix",
                "pnpm run lint -- --fix",
                "pnpm run lint --fix",
                "yarn run lint -- --fix",
                "yarn run lint --fix",
            ];

            for candidate in candidates {
                let mut pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("lint:fix".into(), candidate.to_owned()),
                        ("lint".into(), "eslint .".into()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

                assert_eq!(pkg.scripts, None);

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".to_owned(),
                            TaskConfig {
                                command: Some(TaskCommandArgs::Sequence(string_vec![
                                    "eslint", "."
                                ])),
                                type_of: PlatformType::Node,
                                ..TaskConfig::default()
                            }
                        ),
                        (
                            "lint-fix".to_owned(),
                            TaskConfig {
                                command: Some(TaskCommandArgs::Sequence(string_vec![
                                    "moon",
                                    "run",
                                    "project:lint",
                                    "--",
                                    "--fix"
                                ])),
                                type_of: PlatformType::Node,
                                ..TaskConfig::default()
                            }
                        ),
                    ])
                )
            }
        }

        #[test]
        fn handles_env_vars() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("build".into(), "webpack build".into()),
                    (
                        "build:dev".into(),
                        "NODE_ENV=development npm run build -- --stats".into(),
                    ),
                    (
                        "build:prod".into(),
                        "NODE_ENV=production yarn run build".into(),
                    ),
                    (
                        "build:staging".into(),
                        "NODE_ENV=staging pnpm run build --mode production".into(),
                    ),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "webpack", "build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build-dev".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:build",
                                "--",
                                "--stats"
                            ])),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "development".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build-prod".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build-staging".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:build",
                                "--",
                                "--mode",
                                "production"
                            ])),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "staging".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            )
        }
    }

    mod life_cycle {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn rewrites_run_commands() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("build".into(), "babel .".into()),
                    ("lint".into(), "eslint .".into()),
                    ("test".into(), "jest .".into()),
                    ("preversion".into(), "npm run lint && npm run test".into()),
                    ("version".into(), "npm run build".into()),
                    (
                        "postversion".into(),
                        "npm ci && git add package-lock.json".into(),
                    ),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(
                pkg.scripts,
                Some(BTreeMap::from([
                    (
                        "preversion".to_owned(),
                        "moon run project:lint && moon run project:test".to_owned()
                    ),
                    ("version".to_owned(), "moon run project:build".to_owned()),
                    (
                        "postversion".to_owned(),
                        "npm ci && git add package-lock.json".to_owned()
                    ),
                ]))
            );

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["babel", "."])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["eslint", "."])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["jest", "."])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    )
                ])
            )
        }
    }

    mod complex_examples {
        use super::*;
        use pretty_assertions::assert_eq;

        // https://github.com/babel/babel/blob/main/package.json
        #[test]
        fn babel() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("postinstall".into(), "husky install".into()),
                    ("bootstrap".into(), "make bootstrap".into()),
                    ("codesandbox:build".into(), "make build-no-bundle".into()),
                    ("build".into(), "make build".into()),
                    ("fix".into(), "make fix".into()),
                    ("lint".into(), "make lint".into()),
                    ("test".into(), "make test".into()),
                    (
                        "version".into(),
                        "yarn --immutable-cache && git add yarn.lock".into(),
                    ),
                    ("test:esm".into(), "node test/esm/index.js".into()),
                    (
                        "test:runtime:generate-absolute-runtime".into(),
                        "node test/runtime-integration/generate-absolute-runtime.cjs".into(),
                    ),
                    (
                        "test:runtime:bundlers".into(),
                        "node test/runtime-integration/bundlers.cjs".into(),
                    ),
                    (
                        "test:runtime:node".into(),
                        "node test/runtime-integration/node.cjs".into(),
                    ),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(
                pkg.scripts,
                Some(BTreeMap::from([
                    ("postinstall".to_owned(), "husky install".to_owned()),
                    (
                        "version".to_owned(),
                        "yarn --immutable-cache && git add yarn.lock".to_owned()
                    )
                ]))
            );

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "bootstrap".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "make",
                                "bootstrap"
                            ])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["make", "build"])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "codesandbox-build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "make",
                                "build-no-bundle"
                            ])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "fix".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["make", "fix"])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["make", "lint"])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec!["make", "test"])),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-esm".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "test/esm/index.js"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-bundlers".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "test/runtime-integration/bundlers.cjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-generate-absolute-runtime".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "test/runtime-integration/generate-absolute-runtime.cjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-node".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "test/runtime-integration/node.cjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            );
        }

        // https://github.com/milesj/packemon/blob/master/package.json
        #[test]
        fn packemon() {
            let mut pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("build".into(), "yarn run packemon build".into()),
                        ("check".into(), "yarn run type && yarn run test && yarn run lint".into()),
                        ("clean".into(), "yarn run packemon clean".into()),
                        ("commit".into(), "yarn install && git add yarn.lock".into()),
                        ("coverage".into(), "yarn run test --coverage".into()),
                        ("create-config".into(), "create-config".into()),
                        ("docs".into(), "cd website && yarn run start".into()),
                        ("format".into(), "prettier".into()),
                        ("lint".into(), "eslint".into()),
                        ("packup".into(), "NODE_ENV=production yarn run packemon build --addEngines --addExports --declaration".into()),
                        ("packemon".into(), "node ./packages/packemon/cjs/bin.cjs".into()),
                        ("prerelease".into(), "yarn run clean && yarn run setup && yarn run packup && yarn run check".into()),
                        ("release".into(), "yarn run prerelease && run-script lerna-release".into()),
                        ("setup".into(), "yarn dlx --package packemon@latest --package typescript --quiet packemon build".into()),
                        ("test".into(), "jest".into()),
                        ("type".into(), "typescript --build".into()),
                        ("validate".into(), "yarn run packemon validate".into()),
                    ])),
                    ..PackageJson::default()
                };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "check-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:type",
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "check-dep2".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:test",
                            ])),
                            deps: Some(string_vec!["~:check-dep1"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "check".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:lint",
                            ])),
                            deps: Some(string_vec!["~:check-dep2"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "clean".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "clean"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "commit-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "yarn", "install"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "commit".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "git",
                                "add",
                                "yarn.lock"
                            ])),
                            deps: Some(string_vec!["~:commit-dep1"]),
                            type_of: PlatformType::System,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "coverage".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:test",
                                "--",
                                "--coverage"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "create-config".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("create-config".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "format".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("prettier".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("eslint".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "packup".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "build",
                                "--addEngines",
                                "--addExports",
                                "--declaration"
                            ])),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "packemon".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./packages/packemon/cjs/bin.cjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:clean"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep2".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:setup"
                            ])),
                            deps: Some(string_vec!["~:prerelease-dep1"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep3".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:packup"
                            ])),
                            deps: Some(string_vec!["~:prerelease-dep2"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "prerelease".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:check"
                            ])),
                            deps: Some(string_vec!["~:prerelease-dep3"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "release".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "run-script",
                                "lerna-release"
                            ])),
                            deps: Some(string_vec!["~:prerelease"]),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "setup".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "yarn",
                                "dlx",
                                "--package",
                                "packemon@latest",
                                "--package",
                                "typescript",
                                "--quiet",
                                "packemon",
                                "build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("jest".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "type".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "typescript",
                                "--build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "validate".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "validate"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            )
        }

        // https://github.com/prettier/prettier/blob/main/package.json
        #[test]
        fn prettier() {
            let mut pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("prepublishOnly".into(), "echo \"Error: must publish from dist/\" && exit 1".into()),
                        ("test".into(), "jest".into()),
                        ("test:dev-package".into(), "cross-env INSTALL_PACKAGE=1 jest".into()),
                        ("test:dist".into(), "cross-env NODE_ENV=production jest".into()),
                        ("test:dist-standalone".into(), "cross-env NODE_ENV=production TEST_STANDALONE=1 jest".into()),
                        ("test:integration".into(), "jest tests/integration".into()),
                        ("test:dist-lint".into(), "eslint --no-eslintrc --no-ignore --no-inline-config --config=./scripts/bundle-eslint-config.cjs \"dist/**/*.{js,mjs}\"".into()),
                        ("perf".into(), "yarn run build && cross-env NODE_ENV=production node ./dist/bin-prettier.js".into()),
                        ("perf:inspect".into(), "yarn run build && cross-env NODE_ENV=production node --inspect-brk ./dist/bin-prettier.js".into()),
                        ("perf:benchmark".into(), "yarn run perf --debug-benchmark".into()),
                        ("lint".into(), "run-p lint:*".into()),
                        ("lint:typecheck".into(), "tsc".into()),
                        ("lint:eslint".into(), "cross-env EFF_NO_LINK_RULES=true eslint . --format friendly".into()),
                        ("lint:changelog".into(), "node ./scripts/lint-changelog.mjs".into()),
                        ("lint:prettier".into(), "prettier . \"!test*\" --check".into()),
                        ("lint:spellcheck".into(), "cspell --no-progress --relative --dot --gitignore".into()),
                        ("lint:deps".into(), "node ./scripts/check-deps.mjs".into()),
                        ("lint:actionlint".into(), "node-actionlint".into()),
                        ("fix:eslint".into(), "yarn run lint:eslint --fix".into()),
                        ("fix:prettier".into(), "yarn run lint:prettier --write".into()),
                        ("build".into(), "node ./scripts/build/build.mjs".into()),
                        ("build:website".into(), "node ./scripts/build-website.mjs".into()),
                        ("vendors:bundle".into(), "node ./scripts/vendors/bundle-vendors.mjs".into()),
                    ])),
                    ..PackageJson::default()
                };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(
                pkg.scripts,
                Some(BTreeMap::from([(
                    "prepublishOnly".to_owned(),
                    "echo \"Error: must publish from dist/\" && exit 1".to_owned()
                )]))
            );

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "run-p", "lint:*"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-actionlint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("node-actionlint".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-changelog".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./scripts/lint-changelog.mjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-deps".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./scripts/check-deps.mjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-eslint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "eslint",
                                ".",
                                "--format",
                                "friendly"
                            ])),
                            env: Some(HashMap::from([(
                                "EFF_NO_LINK_RULES".to_owned(),
                                "true".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-prettier".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "prettier", ".", "!test*", "--check"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-spellcheck".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cspell",
                                "--no-progress",
                                "--relative",
                                "--dot",
                                "--gitignore"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "lint-typecheck".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("tsc".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "fix-eslint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:lint-eslint",
                                "--",
                                "--fix"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "fix-prettier".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:lint-prettier",
                                "--",
                                "--write"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./scripts/build/build.mjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "build-website".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./scripts/build-website.mjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "perf".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "node",
                                "./dist/bin-prettier.js"
                            ])),
                            deps: Some(string_vec!["~:perf-dep1"]),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "perf-benchmark".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:perf",
                                "--",
                                "--debug-benchmark"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "perf-inspect".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "node",
                                "--inspect-brk",
                                "./dist/bin-prettier.js"
                            ])),
                            deps: Some(string_vec!["~:perf-inspect-dep1"]),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "perf-inspect-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "perf-dep1".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("jest".to_owned())),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-dev-package".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "jest"
                            ])),
                            env: Some(HashMap::from([(
                                "INSTALL_PACKAGE".to_owned(),
                                "1".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-dist".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "jest"
                            ])),
                            env: Some(HashMap::from([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-dist-lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "eslint",
                                "--no-eslintrc",
                                "--no-ignore",
                                "--no-inline-config",
                                "--config=./scripts/bundle-eslint-config.cjs",
                                "dist/**/*.{js,mjs}"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-dist-standalone".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "cross-env",
                                "jest"
                            ])),
                            env: Some(HashMap::from([
                                ("TEST_STANDALONE".to_owned(), "1".to_owned()),
                                ("NODE_ENV".to_owned(), "production".to_owned())
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "test-integration".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "jest",
                                "tests/integration"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                    (
                        "vendors-bundle".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::Sequence(string_vec![
                                "node",
                                "./scripts/vendors/bundle-vendors.mjs"
                            ])),
                            type_of: PlatformType::Node,
                            ..TaskConfig::default()
                        }
                    ),
                ])
            );
        }
    }
}
