use moon_lang_node::package::PackageJson;
use moon_project_node::task::{
    convert_script_to_task, create_tasks_from_scripts, should_run_in_ci,
};
use moon_task::{Task, TaskType};
use moon_utils::string_vec;
use std::collections::{BTreeMap, HashMap};

mod should_run_in_ci {
    use super::*;

    #[test]
    fn handles_watch_mode() {
        assert!(!should_run_in_ci("packemon build --watch"));
        assert!(!should_run_in_ci("rollup --watch"));
        assert!(!should_run_in_ci("tsc --watch"));
    }

    #[test]
    fn handles_gatsby() {
        // yes
        assert!(should_run_in_ci("gatsby --version"));
        assert!(should_run_in_ci("gatsby --help"));
        assert!(should_run_in_ci("gatsby build"));
        assert!(should_run_in_ci("gatsby info"));
        assert!(should_run_in_ci("npx gatsby build"));

        // no
        assert!(!should_run_in_ci("gatsby dev"));
        assert!(!should_run_in_ci("gatsby develop"));
        assert!(!should_run_in_ci("gatsby new"));
        assert!(!should_run_in_ci("gatsby serve"));
        assert!(!should_run_in_ci("gatsby repl"));
    }

    #[test]
    fn handles_nextjs() {
        // yes
        assert!(should_run_in_ci("next --version"));
        assert!(should_run_in_ci("next --help"));
        assert!(should_run_in_ci("next build"));
        assert!(should_run_in_ci("next export"));
        assert!(should_run_in_ci("npx next build"));

        // no
        assert!(!should_run_in_ci("next dev"));
        assert!(!should_run_in_ci("next start"));
    }

    #[test]
    fn handles_parcel() {
        // yes
        assert!(should_run_in_ci("parcel --version"));
        assert!(should_run_in_ci("parcel --help"));
        assert!(should_run_in_ci("parcel build"));
        assert!(should_run_in_ci("npx parcel build"));

        // no
        assert!(!should_run_in_ci("parcel ./src/index.ts"));
        assert!(!should_run_in_ci("parcel serve index.js"));
        assert!(!should_run_in_ci("parcel watch"));
        assert!(!should_run_in_ci("npx parcel"));
    }

    #[test]
    fn handles_react_scripts() {
        // yes
        assert!(should_run_in_ci("react-scripts --version"));
        assert!(should_run_in_ci("react-scripts --help"));
        assert!(should_run_in_ci("react-scripts build"));
        assert!(should_run_in_ci("react-scripts eject"));
        assert!(should_run_in_ci("npx react-scripts build"));

        // no
        assert!(!should_run_in_ci("react-scripts start"));
        assert!(!should_run_in_ci("react-scripts test --watch"));
    }

    #[test]
    fn handles_snowpack() {
        // yes
        assert!(should_run_in_ci("snowpack --version"));
        assert!(should_run_in_ci("snowpack --help"));
        assert!(should_run_in_ci("snowpack build"));
        assert!(should_run_in_ci("npx snowpack build"));

        // no
        assert!(!should_run_in_ci("snowpack dev"));
    }

    #[test]
    fn handles_vite() {
        // yes
        assert!(should_run_in_ci("vite --version"));
        assert!(should_run_in_ci("vite --help"));
        assert!(should_run_in_ci("vite build"));
        assert!(should_run_in_ci("vite optimize"));
        assert!(should_run_in_ci("npx vite build"));

        // no
        assert!(!should_run_in_ci("vite --watch"));
        assert!(!should_run_in_ci("vite"));
        assert!(!should_run_in_ci("vite dev"));
        assert!(!should_run_in_ci("vite serve"));
        assert!(!should_run_in_ci("vite preview"));
        assert!(!should_run_in_ci("npx vite"));
        assert!(!should_run_in_ci("npx vite dev"));
    }

    #[test]
    fn handles_webpack() {
        // yes
        assert!(should_run_in_ci("webpack --version"));
        assert!(should_run_in_ci("webpack --help"));
        assert!(should_run_in_ci("webpack build"));
        assert!(should_run_in_ci("webpack bundle"));
        assert!(should_run_in_ci("webpack info"));
        assert!(should_run_in_ci("npx webpack build"));

        // no
        assert!(!should_run_in_ci("webpack --entry"));
        assert!(!should_run_in_ci("webpack --watch"));
        assert!(!should_run_in_ci("webpack"));
        assert!(!should_run_in_ci("webpack s"));
        assert!(!should_run_in_ci("webpack serve"));
        assert!(!should_run_in_ci("webpack server"));
        assert!(!should_run_in_ci("webpack w"));
        assert!(!should_run_in_ci("webpack watch"));
        assert!(!should_run_in_ci("npx webpack serve"));
    }
}

mod convert_script_to_task {
    use super::*;

    mod script_files {
        use super::*;

        #[test]
        fn handles_bash() {
            let task =
                convert_script_to_task("project:task", "script", "bash scripts/setup.sh").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "bash".to_owned(),
                    args: string_vec!["scripts/setup.sh"],
                    type_of: TaskType::System,
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn handles_bash_without_command() {
            let task =
                convert_script_to_task("project:task", "script", "scripts/setup.sh").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "bash".to_owned(),
                    args: string_vec!["scripts/setup.sh"],
                    type_of: TaskType::System,
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn handles_node() {
            let task =
                convert_script_to_task("project:task", "script", "node scripts/test.js").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "node".to_owned(),
                    args: string_vec!["scripts/test.js"],
                    type_of: TaskType::Node,
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn handles_node_without_command() {
            let candidates = ["scripts/test.js", "scripts/test.cjs", "scripts/test.mjs"];

            for candidate in candidates {
                let task = convert_script_to_task("project:task", "script", candidate).unwrap();

                assert_eq!(
                    task,
                    Task {
                        command: "node".to_owned(),
                        args: string_vec![candidate],
                        type_of: TaskType::Node,
                        ..Task::new("project:task")
                    }
                )
            }
        }
    }

    mod env_vars {
        use super::*;

        #[test]
        fn extracts_single_var() {
            let task =
                convert_script_to_task("project:task", "script", "KEY=VALUE yarn install").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    args: string_vec!["install"],
                    env: HashMap::from([("KEY".to_owned(), "VALUE".to_owned())]),
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn extracts_multiple_vars() {
            let task = convert_script_to_task(
                "project:task",
                "script",
                "KEY1=VAL1 KEY2=VAL2 yarn install",
            )
            .unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    args: string_vec!["install"],
                    env: HashMap::from([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ]),
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn handles_semicolons() {
            let task = convert_script_to_task(
                "project:task",
                "script",
                "KEY1=VAL1; KEY2=VAL2; yarn install",
            )
            .unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    args: string_vec!["install"],
                    env: HashMap::from([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ]),
                    ..Task::new("project:task")
                }
            )
        }

        #[test]
        fn handles_quoted_values() {
            let task =
                convert_script_to_task("project:task", "script", "NODE_OPTIONS='-f -b' yarn")
                    .unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    env: HashMap::from([("NODE_OPTIONS".to_owned(), "-f -b".to_owned())]),
                    ..Task::new("project:task")
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
                let task = convert_script_to_task(
                    "project:task",
                    "script",
                    &format!("tool build {} {}", candidate.0, candidate.1),
                )
                .unwrap();

                assert_eq!(
                    task,
                    Task {
                        command: "tool".to_owned(),
                        args: string_vec!["build", candidate.0, candidate.1],
                        outputs: string_vec![candidate.2],
                        ..Task::new("project:task")
                    }
                )
            }
        }

        #[should_panic(expected = "NoParentOutput(\"../parent/dir\", \"project:task\")")]
        #[test]
        fn fails_on_parent_relative() {
            convert_script_to_task("project:task", "script", "build --out ../parent/dir").unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"/abs/dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute() {
            convert_script_to_task("project:task", "script", "build --out /abs/dir").unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"C:\\\\abs\\\\dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute_windows() {
            convert_script_to_task("project:task", "script", "build --out C:\\\\abs\\\\dir")
                .unwrap();
        }
    }
}

mod create_tasks_from_scripts {
    use super::*;

    #[test]
    fn ignores_unsupported_syntax() {
        let pkg = PackageJson {
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

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

        assert!(tasks.is_empty());
    }

    #[test]
    fn renames_to_ids() {
        let pkg = PackageJson {
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

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

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
        let pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("test".into(), "jest .".into()),
                ("lint".into(), "eslint src/**/* .".into()),
                ("typecheck".into(), "tsc --build".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "test".to_owned(),
                    Task {
                        command: "jest".to_owned(),
                        args: string_vec!["."],
                        ..Task::new("project:test")
                    }
                ),
                (
                    "lint".to_owned(),
                    Task {
                        command: "eslint".to_owned(),
                        args: string_vec!["src/**/*", "."],
                        ..Task::new("project:lint")
                    }
                ),
                (
                    "typecheck".to_owned(),
                    Task {
                        command: "tsc".to_owned(),
                        args: string_vec!["--build"],
                        ..Task::new("project:typecheck")
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
            let pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("pretest".into(), "do something".into()),
                    ("posttest".into(), "do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "pretest".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["something"],
                            ..Task::new("project:pretest")
                        }
                    ),
                    (
                        "posttest".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["another"],
                            deps: string_vec!["~:test"],
                            ..Task::new("project:posttest")
                        }
                    ),
                    (
                        "test".to_owned(),
                        Task {
                            command: "jest".to_owned(),
                            args: string_vec!["."],
                            deps: string_vec!["~:pretest"],
                            ..Task::new("project:test")
                        }
                    ),
                ])
            )
        }

        #[test]
        fn supports_multiple_pre_via_andand() {
            let pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("pretest".into(), "do something && do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "pretest-dep1".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["something"],
                            ..Task::new("project:pretest-dep1")
                        }
                    ),
                    (
                        "pretest".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["another"],
                            deps: string_vec!["~:pretest-dep1"],
                            ..Task::new("project:pretest")
                        }
                    ),
                    (
                        "test".to_owned(),
                        Task {
                            command: "jest".to_owned(),
                            args: string_vec!["."],
                            deps: string_vec!["~:pretest"],
                            ..Task::new("project:test")
                        }
                    )
                ])
            )
        }

        #[test]
        fn supports_multiple_post_via_andand() {
            let pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".into(), "jest .".into()),
                    ("posttest".into(), "do something && do another".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "posttest-dep1".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["something"],
                            ..Task::new("project:posttest-dep1")
                        }
                    ),
                    (
                        "posttest".to_owned(),
                        Task {
                            command: "do".to_owned(),
                            args: string_vec!["another"],
                            deps: string_vec!["~:posttest-dep1", "~:test"],
                            ..Task::new("project:posttest")
                        }
                    ),
                    (
                        "test".to_owned(),
                        Task {
                            command: "jest".to_owned(),
                            args: string_vec!["."],
                            ..Task::new("project:test")
                        }
                    ),
                ])
            )
        }

        #[test]
        fn handles_pre_within_script() {
            let pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("release".into(), "npm run prerelease && npm publish".into()),
                    ("prerelease".into(), "webpack build".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "prerelease".to_owned(),
                        Task {
                            command: "webpack".to_owned(),
                            args: string_vec!["build"],
                            ..Task::new("project:prerelease")
                        }
                    ),
                    (
                        "release".to_owned(),
                        Task {
                            command: "npm".to_owned(),
                            args: string_vec!["publish"],
                            deps: string_vec!["~:prerelease"],
                            ..Task::new("project:release")
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
            let pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("lint".into(), "eslint .".into()),
                    ("lint:fix".into(), "npm run invalid -- --fix".into()),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([(
                    "lint".to_owned(),
                    Task {
                        command: "eslint".to_owned(),
                        args: string_vec!["."],
                        ..Task::new("project:lint")
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
                let pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("lint".into(), "eslint .".into()),
                        ("lint:fix".into(), candidate.to_owned()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".to_owned(),
                            Task {
                                command: "eslint".to_owned(),
                                args: string_vec!["."],
                                ..Task::new("project:lint")
                            }
                        ),
                        (
                            "lint-fix".to_owned(),
                            Task {
                                command: "moon".to_owned(),
                                args: string_vec!["run", "project:lint", "--"],
                                ..Task::new("project:lint-fix")
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
                let pkg = PackageJson {
                    scripts: Some(BTreeMap::from([
                        ("lint:fix".into(), candidate.to_owned()),
                        ("lint".into(), "eslint .".into()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".to_owned(),
                            Task {
                                command: "eslint".to_owned(),
                                args: string_vec!["."],
                                ..Task::new("project:lint")
                            }
                        ),
                        (
                            "lint-fix".to_owned(),
                            Task {
                                command: "moon".to_owned(),
                                args: string_vec!["run", "project:lint", "--", "--fix"],
                                ..Task::new("project:lint-fix")
                            }
                        ),
                    ])
                )
            }
        }

        #[test]
        fn handles_env_vars() {
            let pkg = PackageJson {
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

            let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "build".to_owned(),
                        Task {
                            command: "webpack".to_owned(),
                            args: string_vec!["build"],
                            ..Task::new("project:build")
                        }
                    ),
                    (
                        "build-dev".to_owned(),
                        Task {
                            command: "moon".to_owned(),
                            args: string_vec!["run", "project:build", "--", "--stats"],
                            env: HashMap::from([("NODE_ENV".to_owned(), "development".to_owned())]),
                            ..Task::new("project:build-dev")
                        }
                    ),
                    (
                        "build-prod".to_owned(),
                        Task {
                            command: "moon".to_owned(),
                            args: string_vec!["run", "project:build", "--"],
                            env: HashMap::from([("NODE_ENV".to_owned(), "production".to_owned())]),
                            ..Task::new("project:build-prod")
                        }
                    ),
                    (
                        "build-staging".to_owned(),
                        Task {
                            command: "moon".to_owned(),
                            args: string_vec!["run", "project:build", "--", "--mode", "production"],
                            env: HashMap::from([("NODE_ENV".to_owned(), "staging".to_owned())]),
                            ..Task::new("project:build-staging")
                        }
                    ),
                ])
            )
        }
    }
}

mod complex_examples {
    use super::*;
    use pretty_assertions::assert_eq;

    // https://github.com/babel/babel/blob/main/package.json
    #[test]
    fn babel() {
        let pkg = PackageJson {
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

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "bootstrap".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["bootstrap"],
                        type_of: TaskType::System,
                        ..Task::new("project:bootstrap")
                    }
                ),
                (
                    "build".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["build"],
                        type_of: TaskType::System,
                        ..Task::new("project:build")
                    }
                ),
                (
                    "codesandbox-build".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["build-no-bundle"],
                        type_of: TaskType::System,
                        ..Task::new("project:codesandbox-build")
                    }
                ),
                (
                    "fix".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["fix"],
                        type_of: TaskType::System,
                        ..Task::new("project:fix")
                    }
                ),
                (
                    "lint".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["lint"],
                        type_of: TaskType::System,
                        ..Task::new("project:lint")
                    }
                ),
                (
                    "test".to_owned(),
                    Task {
                        command: "make".to_owned(),
                        args: string_vec!["test"],
                        type_of: TaskType::System,
                        ..Task::new("project:test")
                    }
                ),
                (
                    "test-esm".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["test/esm/index.js"],
                        ..Task::new("project:test-esm")
                    }
                ),
                (
                    "test-runtime-bundlers".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["test/runtime-integration/bundlers.cjs"],
                        ..Task::new("project:test-runtime-bundlers")
                    }
                ),
                (
                    "test-runtime-generate-absolute-runtime".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["test/runtime-integration/generate-absolute-runtime.cjs"],
                        ..Task::new("project:test-runtime-generate-absolute-runtime")
                    }
                ),
                (
                    "test-runtime-node".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["test/runtime-integration/node.cjs"],
                        ..Task::new("project:test-runtime-node")
                    }
                ),
            ])
        );
    }

    // https://github.com/milesj/packemon/blob/master/package.json
    #[test]
    fn packemon() {
        let pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                ("build".into(), "yarn run packemon build".into()),
                ("check".into(), "yarn run type && yarn run test && yarn run lint".into()),
                ("clean".into(), "yarn run packemon clean".into()),
                ("commit".into(), "yarn install && git add yarn.lock".into()),
                ("coverage".into(), "yarn run test --coverage".into()),
                ("create-config".into(), "beemo create-config".into()),
                ("docs".into(), "cd website && yarn run start".into()),
                ("format".into(), "beemo prettier".into()),
                ("lint".into(), "beemo eslint".into()),
                ("packup".into(), "NODE_ENV=production yarn run packemon build --addEngines --addExports --declaration".into()),
                ("packemon".into(), "node ./packages/packemon/cjs/bin.cjs".into()),
                ("prerelease".into(), "yarn run clean && yarn run setup && yarn run packup && yarn run check".into()),
                ("release".into(), "yarn run prerelease && beemo run-script lerna-release".into()),
                ("setup".into(), "yarn dlx --package packemon@latest --package typescript --quiet packemon build".into()),
                ("test".into(), "beemo jest".into()),
                ("type".into(), "beemo typescript --build".into()),
                ("validate".into(), "yarn run packemon validate".into()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "build".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:packemon", "--", "build"],
                        ..Task::new("project:build")
                    }
                ),
                (
                    "check-dep1".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:type", "--"],
                        ..Task::new("project:check-dep1")
                    }
                ),
                (
                    "check-dep2".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:test", "--"],
                        deps: string_vec!["~:check-dep1"],
                        ..Task::new("project:check-dep2")
                    }
                ),
                (
                    "check".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:lint", "--"],
                        deps: string_vec!["~:check-dep2"],
                        ..Task::new("project:check")
                    }
                ),
                (
                    "clean".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:packemon", "--", "clean"],
                        ..Task::new("project:clean")
                    }
                ),
                (
                    "commit-dep1".to_owned(),
                    Task {
                        command: "yarn".to_owned(),
                        args: string_vec!["install"],
                        ..Task::new("project:commit-dep1")
                    }
                ),
                (
                    "commit".to_owned(),
                    Task {
                        command: "git".to_owned(),
                        args: string_vec!["add", "yarn.lock"],
                        deps: string_vec!["~:commit-dep1"],
                        type_of: TaskType::System,
                        ..Task::new("project:commit")
                    }
                ),
                (
                    "coverage".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:test", "--", "--coverage"],
                        ..Task::new("project:coverage")
                    }
                ),
                (
                    "create-config".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["create-config"],
                        ..Task::new("project:create-config")
                    }
                ),
                (
                    "format".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["prettier"],
                        ..Task::new("project:format")
                    }
                ),
                (
                    "lint".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["eslint"],
                        ..Task::new("project:lint")
                    }
                ),
                (
                    "packup".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec![
                            "run",
                            "project:packemon",
                            "--",
                            "build",
                            "--addEngines",
                            "--addExports",
                            "--declaration"
                        ],
                        env: HashMap::from([("NODE_ENV".to_owned(), "production".to_owned())]),
                        ..Task::new("project:packup")
                    }
                ),
                (
                    "packemon".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./packages/packemon/cjs/bin.cjs"],
                        ..Task::new("project:packemon")
                    }
                ),
                (
                    "prerelease-dep1".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:clean", "--"],
                        ..Task::new("project:prerelease-dep1")
                    }
                ),
                (
                    "prerelease-dep2".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:setup", "--"],
                        deps: string_vec!["~:prerelease-dep1"],
                        ..Task::new("project:prerelease-dep2")
                    }
                ),
                (
                    "prerelease-dep3".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:packup", "--"],
                        deps: string_vec!["~:prerelease-dep2"],
                        ..Task::new("project:prerelease-dep3")
                    }
                ),
                (
                    "prerelease".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:check", "--"],
                        deps: string_vec!["~:prerelease-dep3"],
                        ..Task::new("project:prerelease")
                    }
                ),
                (
                    "release".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["run-script", "lerna-release"],
                        deps: string_vec!["~:prerelease"],
                        ..Task::new("project:release")
                    }
                ),
                (
                    "setup".to_owned(),
                    Task {
                        command: "yarn".to_owned(),
                        args: string_vec![
                            "dlx",
                            "--package",
                            "packemon@latest",
                            "--package",
                            "typescript",
                            "--quiet",
                            "packemon",
                            "build"
                        ],
                        ..Task::new("project:setup")
                    }
                ),
                (
                    "test".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["jest"],
                        ..Task::new("project:test")
                    }
                ),
                (
                    "type".to_owned(),
                    Task {
                        command: "beemo".to_owned(),
                        args: string_vec!["typescript", "--build"],
                        ..Task::new("project:type")
                    }
                ),
                (
                    "validate".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:packemon", "--", "validate"],
                        ..Task::new("project:validate")
                    }
                ),
            ])
        )
    }

    // https://github.com/prettier/prettier/blob/main/package.json
    #[test]
    fn prettier() {
        let pkg = PackageJson {
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

        let tasks = create_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "lint".to_owned(),
                    Task {
                        command: "run-p".to_owned(),
                        args: string_vec!["lint:*"],
                        ..Task::new("project:lint")
                    }
                ),
                (
                    "lint-actionlint".to_owned(),
                    Task {
                        command: "node-actionlint".to_owned(),
                        ..Task::new("project:lint-actionlint")
                    }
                ),
                (
                    "lint-changelog".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./scripts/lint-changelog.mjs"],
                        ..Task::new("project:lint-changelog")
                    }
                ),
                (
                    "lint-deps".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./scripts/check-deps.mjs"],
                        ..Task::new("project:lint-deps")
                    }
                ),
                (
                    "lint-eslint".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["eslint", ".", "--format", "friendly"],
                        env: HashMap::from([("EFF_NO_LINK_RULES".to_owned(), "true".to_owned())]),
                        ..Task::new("project:lint-eslint")
                    }
                ),
                (
                    "lint-prettier".to_owned(),
                    Task {
                        command: "prettier".to_owned(),
                        args: string_vec![".", "!test*", "--check"],
                        ..Task::new("project:lint-prettier")
                    }
                ),
                (
                    "lint-spellcheck".to_owned(),
                    Task {
                        command: "cspell".to_owned(),
                        args: string_vec!["--no-progress", "--relative", "--dot", "--gitignore"],
                        ..Task::new("project:lint-spellcheck")
                    }
                ),
                (
                    "lint-typecheck".to_owned(),
                    Task {
                        command: "tsc".to_owned(),
                        ..Task::new("project:lint-typecheck")
                    }
                ),
                (
                    "fix-eslint".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:lint-eslint", "--", "--fix"],
                        ..Task::new("project:fix-eslint")
                    }
                ),
                (
                    "fix-prettier".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:lint-prettier", "--", "--write"],
                        ..Task::new("project:fix-prettier")
                    }
                ),
                (
                    "build".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./scripts/build/build.mjs"],
                        ..Task::new("project:build")
                    }
                ),
                (
                    "build-website".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./scripts/build-website.mjs"],
                        ..Task::new("project:build-website")
                    }
                ),
                (
                    "perf".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["node", "./dist/bin-prettier.js"],
                        deps: string_vec!["~:perf-dep1"],
                        env: HashMap::from([("NODE_ENV".to_owned(), "production".to_owned())]),
                        ..Task::new("project:perf")
                    }
                ),
                (
                    "perf-benchmark".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:perf", "--", "--debug-benchmark"],
                        ..Task::new("project:perf-benchmark")
                    }
                ),
                (
                    "perf-inspect".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["node", "--inspect-brk", "./dist/bin-prettier.js"],
                        deps: string_vec!["~:perf-inspect-dep1"],
                        env: HashMap::from([("NODE_ENV".to_owned(), "production".to_owned())]),
                        ..Task::new("project:perf-inspect")
                    }
                ),
                (
                    "perf-inspect-dep1".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:build", "--"],
                        ..Task::new("project:perf-inspect-dep1")
                    }
                ),
                (
                    "perf-dep1".to_owned(),
                    Task {
                        command: "moon".to_owned(),
                        args: string_vec!["run", "project:build", "--"],
                        ..Task::new("project:perf-dep1")
                    }
                ),
                (
                    "test".to_owned(),
                    Task {
                        command: "jest".to_owned(),
                        ..Task::new("project:test")
                    }
                ),
                (
                    "test-dev-package".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["jest"],
                        env: HashMap::from([("INSTALL_PACKAGE".to_owned(), "1".to_owned())]),
                        ..Task::new("project:test-dev-package")
                    }
                ),
                (
                    "test-dist".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["jest"],
                        env: HashMap::from([("NODE_ENV".to_owned(), "production".to_owned())]),
                        ..Task::new("project:test-dist")
                    }
                ),
                (
                    "test-dist-lint".to_owned(),
                    Task {
                        command: "eslint".to_owned(),
                        args: string_vec![
                            "--no-eslintrc",
                            "--no-ignore",
                            "--no-inline-config",
                            "--config=./scripts/bundle-eslint-config.cjs",
                            "dist/**/*.{js,mjs}"
                        ],
                        ..Task::new("project:test-dist-lint")
                    }
                ),
                (
                    "test-dist-standalone".to_owned(),
                    Task {
                        command: "cross-env".to_owned(),
                        args: string_vec!["jest"],
                        env: HashMap::from([
                            ("TEST_STANDALONE".to_owned(), "1".to_owned()),
                            ("NODE_ENV".to_owned(), "production".to_owned())
                        ]),
                        ..Task::new("project:test-dist-standalone")
                    }
                ),
                (
                    "test-integration".to_owned(),
                    Task {
                        command: "jest".to_owned(),
                        args: string_vec!["tests/integration"],
                        ..Task::new("project:test-integration")
                    }
                ),
                (
                    "vendors-bundle".to_owned(),
                    Task {
                        command: "node".to_owned(),
                        args: string_vec!["./scripts/vendors/bundle-vendors.mjs"],
                        ..Task::new("project:vendors-bundle")
                    }
                ),
            ])
        );
    }
}
