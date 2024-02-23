use moon_common::Id;
use moon_config::{
    NodePackageManager, OutputPath, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PlatformType,
};
use moon_javascript_platform::infer_tasks::*;
use moon_node_lang::PackageJson;
use moon_target::Target;
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;

fn create_tasks_from_scripts(
    project_id: &str,
    package_json: &mut PackageJson,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id, PlatformType::Node, NodePackageManager::Npm);

    parser.parse_scripts(package_json)?;
    parser.update_package(package_json)?;

    Ok(parser.tasks)
}

fn infer_tasks_from_scripts(
    project_id: &str,
    package_json: &PackageJson,
) -> miette::Result<BTreeMap<Id, PartialTaskConfig>> {
    let mut parser = ScriptParser::new(project_id, PlatformType::Node, NodePackageManager::Yarn);

    parser.infer_scripts(package_json)?;

    Ok(parser.tasks)
}

fn create_target_deps<I, V>(list: I) -> Vec<PartialTaskDependency>
where
    I: IntoIterator<Item = V>,
    V: AsRef<str>,
{
    list.into_iter()
        .map(|value| PartialTaskDependency::Target(Target::parse(value.as_ref()).unwrap()))
        .collect()
}

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

    mod package_managers {
        use super::*;

        #[test]
        fn supports_bun() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::ConvertToTask,
                PlatformType::Bun,
                NodePackageManager::Bun,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["bun", "./test.js"])),
                    platform: Some(PlatformType::Bun),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn wraps_bun() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::WrapRunScript,
                PlatformType::Bun,
                NodePackageManager::Bun,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["bun", "run", "script"])),
                    platform: Some(PlatformType::Bun),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn supports_bun_via_node() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Bun,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["node", "./test.js"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn wraps_bun_via_node() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::WrapRunScript,
                PlatformType::Node,
                NodePackageManager::Bun,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["bun", "run", "script"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn supports_npm() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["node", "./test.js"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn wraps_npm() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::WrapRunScript,
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["npm", "run", "script"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn supports_pnpm() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Pnpm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["node", "./test.js"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn wraps_pnpm() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::WrapRunScript,
                PlatformType::Node,
                NodePackageManager::Pnpm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["pnpm", "run", "script"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn supports_yarn() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Yarn,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["node", "./test.js"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }

        #[test]
        fn wraps_yarn() {
            let task = create_task(
                "project:task",
                "script",
                "./test.js",
                TaskContext::WrapRunScript,
                PlatformType::Node,
                NodePackageManager::Yarn,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["yarn", "run", "script"])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
                }
            )
        }
    }

    mod script_files {
        use super::*;

        #[test]
        fn handles_bash() {
            let task = create_task(
                "project:task",
                "script",
                "bash scripts/setup.sh",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec![
                        "bash",
                        "scripts/setup.sh"
                    ])),
                    platform: Some(PlatformType::System),
                    ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec![
                        "bash",
                        "scripts/setup.sh"
                    ])),
                    platform: Some(PlatformType::System),
                    ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec![
                        "node",
                        "scripts/test.js"
                    ])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
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
                    PlatformType::Node,
                    NodePackageManager::Npm,
                )
                .unwrap();

                assert_eq!(
                    task,
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["node", candidate])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["yarn", "install"])),
                    env: Some(FxHashMap::from_iter([(
                        "KEY".to_owned(),
                        "VALUE".to_owned()
                    )])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["yarn", "install"])),
                    env: Some(FxHashMap::from_iter([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::List(string_vec!["yarn", "install"])),
                    env: Some(FxHashMap::from_iter([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
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
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();

            assert_eq!(
                task,
                PartialTaskConfig {
                    command: Some(PartialTaskArgs::String("yarn".to_owned())),
                    env: Some(FxHashMap::from_iter([(
                        "NODE_OPTIONS".to_owned(),
                        "-f -b".to_owned()
                    )])),
                    platform: Some(PlatformType::Node),
                    ..PartialTaskConfig::default()
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
                    PlatformType::Node,
                    NodePackageManager::Npm,
                )
                .unwrap();

                assert_eq!(
                    task,
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec![
                            "tool",
                            "build",
                            candidate.0,
                            candidate.1
                        ])),
                        outputs: Some(vec![OutputPath::ProjectFile(candidate.2.to_owned())]),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                )
            }
        }

        #[should_panic(
            expected = "Task outputs must be project relative and cannot traverse upwards."
        )]
        #[test]
        fn fails_on_parent_relative() {
            create_task(
                "project:task",
                "script",
                "build --out ../parent/dir",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();
        }

        #[should_panic(expected = "Task outputs must be project relative and cannot be absolute.")]
        #[test]
        fn fails_on_absolute() {
            create_task(
                "project:task",
                "script",
                "build --out /abs/dir",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Npm,
            )
            .unwrap();
        }

        #[should_panic(expected = "Task outputs must be project relative and cannot be absolute.")]
        #[test]
        fn fails_on_absolute_windows() {
            create_task(
                "project:task",
                "script",
                "build --out C:\\\\abs\\\\dir",
                TaskContext::ConvertToTask,
                PlatformType::Node,
                NodePackageManager::Npm,
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
                (
                    "postinstall".try_into().unwrap(),
                    "./setup.sh".try_into().unwrap(),
                ),
                (
                    "build:app".try_into().unwrap(),
                    "webpack build --output ./dist".try_into().unwrap(),
                ),
                ("dev".try_into().unwrap(), "webpack dev".try_into().unwrap()),
                ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                (
                    "posttest".try_into().unwrap(),
                    "run-coverage".try_into().unwrap(),
                ),
                (
                    "lint".try_into().unwrap(),
                    "eslint src/**/* .".try_into().unwrap(),
                ),
                (
                    "typecheck".try_into().unwrap(),
                    "tsc --build".try_into().unwrap(),
                ),
            ])),
            ..PackageJson::default()
        };

        let tasks = infer_tasks_from_scripts("project", &pkg).unwrap();

        assert_eq!(
            tasks,
            BTreeMap::from([
                (
                    "build-app".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec![
                            "yarn",
                            "run",
                            "build:app"
                        ])),
                        outputs: Some(vec![OutputPath::ProjectFile("dist".try_into().unwrap())]),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "dev".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["yarn", "run", "dev"])),
                        local: Some(true),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "test".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["yarn", "run", "test"])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "lint".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["yarn", "run", "lint"])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "typecheck".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec![
                            "yarn",
                            "run",
                            "typecheck"
                        ])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
            ])
        )
    }
}

mod create_tasks_from_scripts {
    use super::*;

    #[test]
    fn ignores_unsupported_syntax() {
        let mut pkg = PackageJson {
            scripts: Some(BTreeMap::from([
                (
                    "cd".try_into().unwrap(),
                    "cd website && yarn build".try_into().unwrap(),
                ),
                (
                    "out".try_into().unwrap(),
                    "some-bin > output.log".try_into().unwrap(),
                ),
                (
                    "in".try_into().unwrap(),
                    "output.log < some-bin".try_into().unwrap(),
                ),
                (
                    "pipe".try_into().unwrap(),
                    "ls | grep foo".try_into().unwrap(),
                ),
                ("or".try_into().unwrap(), "foo || bar".try_into().unwrap()),
                ("semi".try_into().unwrap(), "foo ;; bar".try_into().unwrap()),
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
                ("base".try_into().unwrap(), "script".try_into().unwrap()),
                ("foo-bar".try_into().unwrap(), "script".try_into().unwrap()),
                ("foo_bar".try_into().unwrap(), "script".try_into().unwrap()),
                ("foo:bar".try_into().unwrap(), "script".try_into().unwrap()),
                (
                    "foo-bar:baz".try_into().unwrap(),
                    "script".try_into().unwrap(),
                ),
                (
                    "foo_bar:baz".try_into().unwrap(),
                    "script".try_into().unwrap(),
                ),
                (
                    "foo:bar:baz".try_into().unwrap(),
                    "script".try_into().unwrap(),
                ),
                (
                    "foo_bar:baz-qux".try_into().unwrap(),
                    "script".try_into().unwrap(),
                ),
                ("fooBar".try_into().unwrap(), "script".try_into().unwrap()),
            ])),
            ..PackageJson::default()
        };

        let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

        assert_eq!(
            tasks.keys().map(|t| t.to_string()).collect::<Vec<String>>(),
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
                ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                (
                    "lint".try_into().unwrap(),
                    "eslint src/**/* .".try_into().unwrap(),
                ),
                (
                    "typecheck".try_into().unwrap(),
                    "tsc --build".try_into().unwrap(),
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
                    "test".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["jest", "."])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "lint".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec![
                            "eslint", "src/**/*", "."
                        ])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
                (
                    "typecheck".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["tsc", "--build"])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
                    }
                ),
            ])
        )
    }

    mod pre_post {
        use super::*;

        #[test]
        fn creates_pre_and_post() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                    (
                        "pretest".try_into().unwrap(),
                        "do something".try_into().unwrap(),
                    ),
                    (
                        "posttest".try_into().unwrap(),
                        "do another".try_into().unwrap(),
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
                        "pretest".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "something"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "posttest".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "another"])),
                            deps: Some(create_target_deps(["~:test"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["jest", "."])),
                            deps: Some(create_target_deps(["~:pretest"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                ])
            )
        }

        #[test]
        fn supports_multiple_pre_via_andand() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                    (
                        "pretest".try_into().unwrap(),
                        "do something && do another".try_into().unwrap(),
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
                        "pretest-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "something"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "pretest".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "another"])),
                            deps: Some(create_target_deps(["~:pretest-dep1"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["jest", "."])),
                            deps: Some(create_target_deps(["~:pretest"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    )
                ])
            )
        }

        #[test]
        fn supports_multiple_post_via_andand() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                    (
                        "posttest".try_into().unwrap(),
                        "do something && do another".try_into().unwrap(),
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
                        "posttest-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "something"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "posttest".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["do", "another"])),
                            deps: Some(create_target_deps(["~:posttest-dep1", "~:test"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["jest", "."])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                ])
            )
        }

        #[test]
        fn handles_pre_within_script() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    (
                        "release".try_into().unwrap(),
                        "npm run prerelease && npm publish".try_into().unwrap(),
                    ),
                    (
                        "prerelease".try_into().unwrap(),
                        "webpack build".try_into().unwrap(),
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
                        "prerelease".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["webpack", "build"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "release".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["npm", "publish"])),
                            deps: Some(create_target_deps(["~:prerelease"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                ])
            )
        }
    }

    mod pm_run {
        use super::*;

        #[test]
        fn skips_when_pointing_to_an_unknown() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("lint".try_into().unwrap(), "eslint .".try_into().unwrap()),
                    (
                        "lint:fix".try_into().unwrap(),
                        "npm run invalid -- --fix".try_into().unwrap(),
                    ),
                ])),
                ..PackageJson::default()
            };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([(
                    "lint".try_into().unwrap(),
                    PartialTaskConfig {
                        command: Some(PartialTaskArgs::List(string_vec!["eslint", "."])),
                        platform: Some(PlatformType::Node),
                        ..PartialTaskConfig::default()
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
                        ("lint".try_into().unwrap(), "eslint .".try_into().unwrap()),
                        ("lint:fix".try_into().unwrap(), candidate.to_owned()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

                assert_eq!(pkg.scripts, None);

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".try_into().unwrap(),
                            PartialTaskConfig {
                                command: Some(PartialTaskArgs::List(string_vec!["eslint", "."])),
                                platform: Some(PlatformType::Node),
                                ..PartialTaskConfig::default()
                            }
                        ),
                        (
                            "lint-fix".try_into().unwrap(),
                            PartialTaskConfig {
                                command: Some(PartialTaskArgs::List(string_vec![
                                    "moon",
                                    "run",
                                    "project:lint"
                                ])),
                                platform: Some(PlatformType::Node),
                                ..PartialTaskConfig::default()
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
                        ("lint:fix".try_into().unwrap(), candidate.to_owned()),
                        ("lint".try_into().unwrap(), "eslint .".try_into().unwrap()),
                    ])),
                    ..PackageJson::default()
                };

                let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

                assert_eq!(pkg.scripts, None);

                assert_eq!(
                    tasks,
                    BTreeMap::from([
                        (
                            "lint".try_into().unwrap(),
                            PartialTaskConfig {
                                command: Some(PartialTaskArgs::List(string_vec!["eslint", "."])),
                                platform: Some(PlatformType::Node),
                                ..PartialTaskConfig::default()
                            }
                        ),
                        (
                            "lint-fix".try_into().unwrap(),
                            PartialTaskConfig {
                                command: Some(PartialTaskArgs::List(string_vec![
                                    "moon",
                                    "run",
                                    "project:lint",
                                    "--",
                                    "--fix"
                                ])),
                                platform: Some(PlatformType::Node),
                                ..PartialTaskConfig::default()
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
                    (
                        "build".try_into().unwrap(),
                        "webpack build".try_into().unwrap(),
                    ),
                    (
                        "build:dev".try_into().unwrap(),
                        "NODE_ENV=development npm run build -- --stats"
                            .try_into()
                            .unwrap(),
                    ),
                    (
                        "build:prod".try_into().unwrap(),
                        "NODE_ENV=production yarn run build".try_into().unwrap(),
                    ),
                    (
                        "build:staging".try_into().unwrap(),
                        "NODE_ENV=staging pnpm run build --mode production"
                            .try_into()
                            .unwrap(),
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
                        "build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["webpack", "build"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build-dev".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:build",
                                "--",
                                "--stats"
                            ])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "development".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build-prod".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build-staging".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:build",
                                "--",
                                "--mode",
                                "production"
                            ])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "staging".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                ])
            )
        }
    }

    mod life_cycle {
        use super::*;

        #[test]
        fn rewrites_run_commands() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    ("build".try_into().unwrap(), "babel .".try_into().unwrap()),
                    ("lint".try_into().unwrap(), "eslint .".try_into().unwrap()),
                    ("test".try_into().unwrap(), "jest .".try_into().unwrap()),
                    (
                        "preversion".try_into().unwrap(),
                        "npm run lint && npm run test".try_into().unwrap(),
                    ),
                    (
                        "version".try_into().unwrap(),
                        "npm run build".try_into().unwrap(),
                    ),
                    (
                        "postversion".try_into().unwrap(),
                        "npm ci && git add package-lock.json".try_into().unwrap(),
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
                        "build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["babel", "."])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["eslint", "."])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["jest", "."])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    )
                ])
            )
        }
    }

    mod complex_examples {
        use super::*;

        // https://github.com/babel/babel/blob/main/package.json
        #[test]
        fn babel() {
            let mut pkg = PackageJson {
                scripts: Some(BTreeMap::from([
                    (
                        "postinstall".try_into().unwrap(),
                        "husky install".try_into().unwrap(),
                    ),
                    (
                        "bootstrap".try_into().unwrap(),
                        "make bootstrap".try_into().unwrap(),
                    ),
                    (
                        "codesandbox:build".try_into().unwrap(),
                        "make build-no-bundle".try_into().unwrap(),
                    ),
                    (
                        "build".try_into().unwrap(),
                        "make build".try_into().unwrap(),
                    ),
                    ("fix".try_into().unwrap(), "make fix".try_into().unwrap()),
                    ("lint".try_into().unwrap(), "make lint".try_into().unwrap()),
                    ("test".try_into().unwrap(), "make test".try_into().unwrap()),
                    (
                        "version".try_into().unwrap(),
                        "yarn --immutable-cache && git add yarn.lock"
                            .try_into()
                            .unwrap(),
                    ),
                    (
                        "test:esm".try_into().unwrap(),
                        "node test/esm/index.js".try_into().unwrap(),
                    ),
                    (
                        "test:runtime:generate-absolute-runtime".try_into().unwrap(),
                        "node test/runtime-integration/generate-absolute-runtime.cjs"
                            .try_into()
                            .unwrap(),
                    ),
                    (
                        "test:runtime:bundlers".try_into().unwrap(),
                        "node test/runtime-integration/bundlers.cjs"
                            .try_into()
                            .unwrap(),
                    ),
                    (
                        "test:runtime:node".try_into().unwrap(),
                        "node test/runtime-integration/node.cjs".try_into().unwrap(),
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
                        "bootstrap".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["make", "bootstrap"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["make", "build"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "codesandbox-build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "make",
                                "build-no-bundle"
                            ])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "fix".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["make", "fix"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["make", "lint"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["make", "test"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-esm".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "test/esm/index.js"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-bundlers".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "test/runtime-integration/bundlers.cjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-generate-absolute-runtime".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "test/runtime-integration/generate-absolute-runtime.cjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-runtime-node".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "test/runtime-integration/node.cjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
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
                        ("build".try_into().unwrap(), "yarn run packemon build".try_into().unwrap()),
                        ("check".try_into().unwrap(), "yarn run type && yarn run test && yarn run lint".try_into().unwrap()),
                        ("clean".try_into().unwrap(), "yarn run packemon clean".try_into().unwrap()),
                        ("commit".try_into().unwrap(), "yarn install && git add yarn.lock".try_into().unwrap()),
                        ("coverage".try_into().unwrap(), "yarn run test --coverage".try_into().unwrap()),
                        ("create-config".try_into().unwrap(), "create-config".try_into().unwrap()),
                        ("docs".try_into().unwrap(), "cd website && yarn run start".try_into().unwrap()),
                        ("format".try_into().unwrap(), "prettier".try_into().unwrap()),
                        ("lint".try_into().unwrap(), "eslint".try_into().unwrap()),
                        ("packup".try_into().unwrap(), "NODE_ENV=production yarn run packemon build --addEngines --addExports --declaration".try_into().unwrap()),
                        ("packemon".try_into().unwrap(), "node ./packages/packemon/cjs/bin.cjs".try_into().unwrap()),
                        ("prerelease".try_into().unwrap(), "yarn run clean && yarn run setup && yarn run packup && yarn run check".try_into().unwrap()),
                        ("release".try_into().unwrap(), "yarn run prerelease && run-script lerna-release".try_into().unwrap()),
                        ("setup".try_into().unwrap(), "yarn dlx --package packemon@latest --package typescript --quiet packemon build".try_into().unwrap()),
                        ("test".try_into().unwrap(), "jest".try_into().unwrap()),
                        ("type".try_into().unwrap(), "typescript --build".try_into().unwrap()),
                        ("validate".try_into().unwrap(), "yarn run packemon validate".try_into().unwrap()),
                    ])),
                    ..PackageJson::default()
                };

            let tasks = create_tasks_from_scripts("project", &mut pkg).unwrap();

            assert_eq!(pkg.scripts, None);

            assert_eq!(
                tasks,
                BTreeMap::from([
                    (
                        "build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "build"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "check-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:type",
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "check-dep2".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:test",
                            ])),
                            deps: Some(create_target_deps(["~:check-dep1"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "check".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:lint",
                            ])),
                            deps: Some(create_target_deps(["~:check-dep2"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "clean".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "clean"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "commit-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["yarn", "install"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "commit".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "git",
                                "add",
                                "yarn.lock"
                            ])),
                            deps: Some(create_target_deps(["~:commit-dep1"])),
                            platform: Some(PlatformType::System),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "coverage".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:test",
                                "--",
                                "--coverage"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "create-config".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("create-config".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "format".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("prettier".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("eslint".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "packup".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "build",
                                "--addEngines",
                                "--addExports",
                                "--declaration"
                            ])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "packemon".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./packages/packemon/cjs/bin.cjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:clean"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep2".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:setup"
                            ])),
                            deps: Some(create_target_deps(["~:prerelease-dep1"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "prerelease-dep3".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:packup"
                            ])),
                            deps: Some(create_target_deps(["~:prerelease-dep2"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "prerelease".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:check"
                            ])),
                            deps: Some(create_target_deps(["~:prerelease-dep3"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "release".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "run-script",
                                "lerna-release"
                            ])),
                            deps: Some(create_target_deps(["~:prerelease"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "setup".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
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
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("jest".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "type".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "typescript",
                                "--build"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "validate".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:packemon",
                                "--",
                                "validate"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
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
                        ("prepublishOnly".try_into().unwrap(), "echo \"Error: must publish from dist/\" && exit 1".try_into().unwrap()),
                        ("test".try_into().unwrap(), "jest".try_into().unwrap()),
                        ("test:dev-package".try_into().unwrap(), "cross-env INSTALL_PACKAGE=1 jest".try_into().unwrap()),
                        ("test:dist".try_into().unwrap(), "cross-env NODE_ENV=production jest".try_into().unwrap()),
                        ("test:dist-standalone".try_into().unwrap(), "cross-env NODE_ENV=production TEST_STANDALONE=1 jest".try_into().unwrap()),
                        ("test:integration".try_into().unwrap(), "jest tests/integration".try_into().unwrap()),
                        ("test:dist-lint".try_into().unwrap(), "eslint --no-eslintrc --no-ignore --no-inline-config --config=./scripts/bundle-eslint-config.cjs \"dist/**/*.{js,mjs}\"".try_into().unwrap()),
                        ("perf".try_into().unwrap(), "yarn run build && cross-env NODE_ENV=production node ./dist/bin-prettier.js".try_into().unwrap()),
                        ("perf:inspect".try_into().unwrap(), "yarn run build && cross-env NODE_ENV=production node --inspect-brk ./dist/bin-prettier.js".try_into().unwrap()),
                        ("perf:benchmark".try_into().unwrap(), "yarn run perf --debug-benchmark".try_into().unwrap()),
                        ("lint".try_into().unwrap(), "run-p lint:*".try_into().unwrap()),
                        ("lint:typecheck".try_into().unwrap(), "tsc".try_into().unwrap()),
                        ("lint:eslint".try_into().unwrap(), "cross-env EFF_NO_LINK_RULES=true eslint . --format friendly".try_into().unwrap()),
                        ("lint:changelog".try_into().unwrap(), "node ./scripts/lint-changelog.mjs".try_into().unwrap()),
                        ("lint:prettier".try_into().unwrap(), "prettier . \"!test*\" --check".try_into().unwrap()),
                        ("lint:spellcheck".try_into().unwrap(), "cspell --no-progress --relative --dot --gitignore".try_into().unwrap()),
                        ("lint:deps".try_into().unwrap(), "node ./scripts/check-deps.mjs".try_into().unwrap()),
                        ("lint:actionlint".try_into().unwrap(), "node-actionlint".try_into().unwrap()),
                        ("fix:eslint".try_into().unwrap(), "yarn run lint:eslint --fix".try_into().unwrap()),
                        ("fix:prettier".try_into().unwrap(), "yarn run lint:prettier --write".try_into().unwrap()),
                        ("build".try_into().unwrap(), "node ./scripts/build/build.mjs".try_into().unwrap()),
                        ("build:website".try_into().unwrap(), "node ./scripts/build-website.mjs".try_into().unwrap()),
                        ("vendors:bundle".try_into().unwrap(), "node ./scripts/vendors/bundle-vendors.mjs".try_into().unwrap()),
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
                        "lint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["run-p", "lint:*"])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-actionlint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("node-actionlint".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-changelog".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./scripts/lint-changelog.mjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-deps".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./scripts/check-deps.mjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-eslint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "cross-env",
                                "eslint",
                                ".",
                                "--format",
                                "friendly"
                            ])),
                            env: Some(FxHashMap::from_iter([(
                                "EFF_NO_LINK_RULES".to_owned(),
                                "true".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-prettier".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "prettier", ".", "!test*", "--check"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-spellcheck".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "cspell",
                                "--no-progress",
                                "--relative",
                                "--dot",
                                "--gitignore"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "lint-typecheck".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("tsc".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "fix-eslint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:lint-eslint",
                                "--",
                                "--fix"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "fix-prettier".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:lint-prettier",
                                "--",
                                "--write"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./scripts/build/build.mjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "build-website".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./scripts/build-website.mjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "perf".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "cross-env",
                                "node",
                                "./dist/bin-prettier.js"
                            ])),
                            deps: Some(create_target_deps(["~:perf-dep1"])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "perf-benchmark".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:perf",
                                "--",
                                "--debug-benchmark"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "perf-inspect".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "cross-env",
                                "node",
                                "--inspect-brk",
                                "./dist/bin-prettier.js"
                            ])),
                            deps: Some(create_target_deps(["~:perf-inspect-dep1"])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "perf-inspect-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "perf-dep1".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "moon",
                                "run",
                                "project:build"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::String("jest".to_owned())),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-dev-package".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["cross-env", "jest"])),
                            env: Some(FxHashMap::from_iter([(
                                "INSTALL_PACKAGE".to_owned(),
                                "1".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-dist".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["cross-env", "jest"])),
                            env: Some(FxHashMap::from_iter([(
                                "NODE_ENV".to_owned(),
                                "production".to_owned()
                            )])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-dist-lint".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "eslint",
                                "--no-eslintrc",
                                "--no-ignore",
                                "--no-inline-config",
                                "--config=./scripts/bundle-eslint-config.cjs",
                                "dist/**/*.{js,mjs}"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-dist-standalone".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec!["cross-env", "jest"])),
                            env: Some(FxHashMap::from_iter([
                                ("TEST_STANDALONE".to_owned(), "1".to_owned()),
                                ("NODE_ENV".to_owned(), "production".to_owned())
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "test-integration".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "jest",
                                "tests/integration"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                    (
                        "vendors-bundle".try_into().unwrap(),
                        PartialTaskConfig {
                            command: Some(PartialTaskArgs::List(string_vec![
                                "node",
                                "./scripts/vendors/bundle-vendors.mjs"
                            ])),
                            platform: Some(PlatformType::Node),
                            ..PartialTaskConfig::default()
                        }
                    ),
                ])
            );
        }
    }
}
