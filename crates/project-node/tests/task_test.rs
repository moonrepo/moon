use moon_project_node::task::{convert_script_to_task, should_run_in_ci};
use moon_task::{Task, TaskType};
use moon_utils::string_vec;
use std::collections::HashMap;

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
            let task = convert_script_to_task("project:task", "bash scripts/setup.sh").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "bash".to_owned(),
                    args: string_vec!["scripts/setup.sh"],
                    type_of: TaskType::System,
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn handles_bash_without_command() {
            let task = convert_script_to_task("project:task", "scripts/setup.sh").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "bash".to_owned(),
                    args: string_vec!["scripts/setup.sh"],
                    type_of: TaskType::System,
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn handles_node() {
            let task = convert_script_to_task("project:task", "node scripts/test.js").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "node".to_owned(),
                    args: string_vec!["scripts/test.js"],
                    type_of: TaskType::Node,
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn handles_node_without_command() {
            let candidates = ["scripts/test.js", "scripts/test.cjs", "scripts/test.mjs"];

            for candidate in candidates {
                let task = convert_script_to_task("project:task", candidate).unwrap();

                assert_eq!(
                    task,
                    Task {
                        command: "node".to_owned(),
                        args: string_vec![candidate],
                        type_of: TaskType::Node,
                        ..Task::new("project:task".to_owned())
                    }
                )
            }
        }
    }

    mod env_vars {
        use super::*;

        #[test]
        fn extracts_single_var() {
            let task = convert_script_to_task("project:task", "KEY=VALUE yarn install").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    args: string_vec!["install"],
                    env: HashMap::from([("KEY".to_owned(), "VALUE".to_owned())]),
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn extracts_multiple_vars() {
            let task =
                convert_script_to_task("project:task", "KEY1=VAL1 KEY2=VAL2 yarn install").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    args: string_vec!["install"],
                    env: HashMap::from([
                        ("KEY1".to_owned(), "VAL1".to_owned()),
                        ("KEY2".to_owned(), "VAL2".to_owned())
                    ]),
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn handles_semicolons() {
            let task = convert_script_to_task("project:task", "KEY1=VAL1; KEY2=VAL2; yarn install")
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
                    ..Task::new("project:task".to_owned())
                }
            )
        }

        #[test]
        fn handles_quoted_values() {
            let task = convert_script_to_task("project:task", "NODE_OPTIONS='-f -b' yarn").unwrap();

            assert_eq!(
                task,
                Task {
                    command: "yarn".to_owned(),
                    env: HashMap::from([("NODE_OPTIONS".to_owned(), "-f -b".to_owned())]),
                    ..Task::new("project:task".to_owned())
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
                    &format!("tool build {} {}", candidate.0, candidate.1),
                )
                .unwrap();

                assert_eq!(
                    task,
                    Task {
                        command: "tool".to_owned(),
                        args: string_vec!["build", candidate.0, candidate.1],
                        outputs: string_vec![candidate.2],
                        ..Task::new("project:task".to_owned())
                    }
                )
            }
        }

        #[should_panic(expected = "NoParentOutput(\"../parent/dir\", \"project:task\")")]
        #[test]
        fn fails_on_parent_relative() {
            convert_script_to_task("project:task", "build --out ../parent/dir").unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"/abs/dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute() {
            convert_script_to_task("project:task", "build --out /abs/dir").unwrap();
        }

        #[should_panic(expected = "NoAbsoluteOutput(\"C:\\\\abs\\\\dir\", \"project:task\")")]
        #[test]
        fn fails_on_absolute_windows() {
            convert_script_to_task("project:task", "build --out C:\\\\abs\\\\dir").unwrap();
        }
    }
}
