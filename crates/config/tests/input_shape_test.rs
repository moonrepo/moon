use moon_common::Id;
use moon_config::{
    FileGroupInput, FileGroupInputFormat, GlobInput, GlobPath, Input, ManifestDepsInput,
    ProjectInput, Uri, test_utils::*,
};
use schematic::RegexSetting;

mod input_shape {
    use super::*;

    mod parse_string {
        use super::*;

        #[test]
        fn converts_backward_slashes() {
            assert_eq!(
                Input::parse("some\\file.txt").unwrap(),
                Input::File(stub_file_input("some/file.txt"))
            );
        }

        #[test]
        fn env_vars() {
            assert_eq!(Input::parse("$VAR").unwrap(), Input::EnvVar("VAR".into()));
            assert_eq!(
                Input::parse("$VAR_*").unwrap(),
                Input::EnvVarGlob("VAR_*".into())
            );
            assert_eq!(
                Input::parse("$VAR_*_SUFFIX").unwrap(),
                Input::EnvVarGlob("VAR_*_SUFFIX".into())
            );
            assert_eq!(
                Input::parse("$*_SUFFIX").unwrap(),
                Input::EnvVarGlob("*_SUFFIX".into())
            );
        }

        #[test]
        fn token_funcs() {
            assert_eq!(
                Input::parse("@group(name)").unwrap(),
                Input::TokenFunc("@group(name)".into())
            );
            assert_eq!(
                Input::parse("@dirs(name)").unwrap(),
                Input::TokenFunc("@dirs(name)".into())
            );
            assert_eq!(
                Input::parse("@files(name)").unwrap(),
                Input::TokenFunc("@files(name)".into())
            );
            assert_eq!(
                Input::parse("@globs(name)").unwrap(),
                Input::TokenFunc("@globs(name)".into())
            );
            assert_eq!(
                Input::parse("@root(name)").unwrap(),
                Input::TokenFunc("@root(name)".into())
            );
        }

        #[test]
        fn token_vars() {
            assert_eq!(
                Input::parse("$workspaceRoot").unwrap(),
                Input::TokenVar("$workspaceRoot".into())
            );
            assert_eq!(
                Input::parse("$projectType").unwrap(),
                Input::TokenVar("$projectType".into())
            );
        }

        #[test]
        fn file_protocol() {
            let mut input = stub_file_input("file.txt");
            input.optional = Some(true);

            assert_eq!(
                Input::parse("file://file.txt?optional").unwrap(),
                Input::File(input)
            );

            let mut input = stub_file_input("/file.txt");
            input.optional = Some(false);

            assert_eq!(
                Input::parse("file:///file.txt?optional=false").unwrap(),
                Input::File(input)
            );
        }

        #[test]
        fn file_project_relative() {
            assert_eq!(
                Input::parse("file.rs").unwrap(),
                Input::File(stub_file_input("file.rs"))
            );
            assert_eq!(
                Input::parse("dir/file.rs").unwrap(),
                Input::File(stub_file_input("dir/file.rs"))
            );
            assert_eq!(
                Input::parse("./file.rs").unwrap(),
                Input::File(stub_file_input("file.rs"))
            );
            assert_eq!(
                Input::parse("././dir/file.rs").unwrap(),
                Input::File(stub_file_input("dir/file.rs"))
            );
        }

        #[test]
        fn file_project_relative_protocol() {
            assert_eq!(
                Input::parse("file://file.rs").unwrap(),
                Input::File(stub_file_input("file.rs"))
            );
            assert_eq!(
                Input::parse("file://dir/file.rs").unwrap(),
                Input::File(stub_file_input("dir/file.rs"))
            );
            assert_eq!(
                Input::parse("file://./file.rs").unwrap(),
                Input::File(stub_file_input("file.rs"))
            );
            assert_eq!(
                Input::parse("file://././dir/file.rs").unwrap(),
                Input::File(stub_file_input("dir/file.rs"))
            );
        }

        #[test]
        fn file_workspace_relative() {
            assert_eq!(
                Input::parse("/file.rs").unwrap(),
                Input::File(stub_file_input("/file.rs"))
            );
            assert_eq!(
                Input::parse("/dir/file.rs").unwrap(),
                Input::File(stub_file_input("/dir/file.rs"))
            );

            // With tokens
            assert_eq!(
                Input::parse("/.cache/$projectSource").unwrap(),
                Input::File(stub_file_input("/.cache/$projectSource"))
            );
        }

        #[test]
        fn file_workspace_relative_protocol() {
            assert_eq!(
                Input::parse("file:///file.rs").unwrap(),
                Input::File(stub_file_input("/file.rs"))
            );
            assert_eq!(
                Input::parse("file:///dir/file.rs").unwrap(),
                Input::File(stub_file_input("/dir/file.rs"))
            );

            // With tokens
            assert_eq!(
                Input::parse("file:///.cache/$projectSource").unwrap(),
                Input::File(stub_file_input("/.cache/$projectSource"))
            );
        }

        #[test]
        fn glob_protocol() {
            let mut input = stub_glob_input("file.*");
            input.cache = true;

            assert_eq!(
                Input::parse("glob://file.*?cache").unwrap(),
                Input::Glob(input)
            );

            let mut input = stub_glob_input("/file.*");
            input.cache = false;

            assert_eq!(
                Input::parse("glob:///file.*?cache=false").unwrap(),
                Input::Glob(input)
            );
        }

        #[test]
        fn glob_protocol_supports_all_syntax() {
            for pat in [
                "*.png",
                "ba(r|z).txt",
                "**/{*.{go,rs}}",
                "**/*.{md,txt}",
                "pkg/**/PKGBUILD",
                "dir/{a?c,x?z,foo}",
                "lib/[qa-cX-Z]/*",
                "(?-i)photos/**/*.(?i){jpg,jpeg}",
                "a/<b/**:1,>",
                "file.tsx?",
            ] {
                let mut input = stub_glob_input(pat);
                input.cache = true;

                assert_eq!(
                    Input::Glob(GlobInput {
                        glob: GlobPath(pat.into()),
                        cache: true
                    }),
                    Input::Glob(input)
                );

                let mut input = stub_glob_input(pat);
                input.cache = false;

                assert_eq!(
                    Input::Glob(GlobInput {
                        glob: GlobPath(pat.into()),
                        cache: false
                    }),
                    Input::Glob(input)
                );
            }
        }

        #[test]
        fn glob_project_relative() {
            assert_eq!(
                Input::parse("!file.*").unwrap(),
                Input::Glob(stub_glob_input("!file.*"))
            );
            assert_eq!(
                Input::parse("dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("dir/**/*"))
            );
            assert_eq!(
                Input::parse("./dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("dir/**/*"))
            );

            // With tokens
            assert_eq!(
                Input::parse("$projectSource/**/*").unwrap(),
                Input::Glob(stub_glob_input("$projectSource/**/*"))
            );
        }

        #[test]
        fn glob_project_relative_protocol() {
            assert_eq!(
                Input::parse("glob://!file.*").unwrap(),
                Input::Glob(stub_glob_input("!file.*"))
            );
            assert_eq!(
                Input::parse("glob://dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("dir/**/*"))
            );
            assert_eq!(
                Input::parse("glob://./dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("dir/**/*"))
            );

            // With tokens
            assert_eq!(
                Input::parse("glob://$projectSource/**/*").unwrap(),
                Input::Glob(stub_glob_input("$projectSource/**/*"))
            );
        }

        #[test]
        fn glob_workspace_relative() {
            assert_eq!(
                Input::parse("/!file.*").unwrap(),
                Input::Glob(stub_glob_input("!/file.*"))
            );
            assert_eq!(
                Input::parse("!/file.*").unwrap(),
                Input::Glob(stub_glob_input("!/file.*"))
            );
            assert_eq!(
                Input::parse("/dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("/dir/**/*"))
            );
        }

        #[test]
        fn glob_workspace_relative_protocol() {
            assert_eq!(
                Input::parse("glob:///!file.*").unwrap(),
                Input::Glob(stub_glob_input("!/file.*"))
            );
            assert_eq!(
                Input::parse("glob://!/file.*").unwrap(),
                Input::Glob(stub_glob_input("!/file.*"))
            );
            assert_eq!(
                Input::parse("glob:///dir/**/*").unwrap(),
                Input::Glob(stub_glob_input("/dir/**/*"))
            );
        }

        #[test]
        fn file_group_protocol() {
            assert_eq!(
                Input::parse("group://sources").unwrap(),
                Input::FileGroup(FileGroupInput {
                    group: Id::raw("sources"),
                    ..Default::default()
                })
            );
            assert_eq!(
                Input::parse("group://sources?format=dirs").unwrap(),
                Input::FileGroup(FileGroupInput {
                    group: Id::raw("sources"),
                    format: FileGroupInputFormat::Dirs,
                })
            );
            assert_eq!(
                Input::parse("group://sources?as=root").unwrap(),
                Input::FileGroup(FileGroupInput {
                    group: Id::raw("sources"),
                    format: FileGroupInputFormat::Root,
                })
            );
        }

        #[test]
        fn project_protocol() {
            assert_eq!(
                Input::parse("project://app").unwrap(),
                Input::Project(ProjectInput {
                    project: "app".into(),
                    ..Default::default()
                })
            );
            assert_eq!(
                Input::parse("project://app?filter=src/**&filter=!tests/**/*").unwrap(),
                Input::Project(ProjectInput {
                    project: "app".into(),
                    filter: vec!["src/**".into(), "!tests/**/*".into()],
                    ..Default::default()
                })
            );
            assert_eq!(
                Input::parse("project://app?group=sources").unwrap(),
                Input::Project(ProjectInput {
                    project: "app".into(),
                    group: Some(Id::raw("sources")),
                    ..Default::default()
                })
            );
        }

        #[test]
        fn project_protocol_all() {
            assert_eq!(
                Input::parse("project://^").unwrap(),
                Input::Project(ProjectInput {
                    project: "^".into(),
                    ..Default::default()
                })
            );
            assert_eq!(
                Input::parse("project://^?filter=src/**&filter=!tests/**/*").unwrap(),
                Input::Project(ProjectInput {
                    project: "^".into(),
                    filter: vec!["src/**".into(), "!tests/**/*".into()],
                    ..Default::default()
                })
            );
            assert_eq!(
                Input::parse("project://^?group=sources").unwrap(),
                Input::Project(ProjectInput {
                    project: "^".into(),
                    group: Some(Id::raw("sources")),
                    ..Default::default()
                })
            );
        }

        #[test]
        #[should_panic(expected = "input protocol `unknown://` is not supported")]
        fn errors_for_unknown_protocol() {
            Input::parse("unknown://test").unwrap();
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_for_parent_traversal() {
            Input::parse("../../file.txt").unwrap();
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_for_parent_traversal_inner() {
            Input::parse("dir/../../file.txt").unwrap();
        }
    }

    mod parse_object {
        use super::*;

        #[test]
        fn files() {
            let input: Input = serde_json::from_str(r#""file.txt""#).unwrap();

            assert_eq!(input, Input::File(stub_file_input("file.txt")));

            let input: Input = serde_json::from_str(r#"{ "file": "file.txt" }"#).unwrap();

            assert_eq!(input, Input::File(stub_file_input("file.txt")));

            let input: Input =
                serde_json::from_str(r#"{ "file": "dir/file.txt", "optional": true }"#).unwrap();

            assert_eq!(
                input,
                Input::File({
                    let mut inner = stub_file_input("dir/file.txt");
                    inner.optional = Some(true);
                    inner
                })
            );

            let input: Input = serde_json::from_str(
                r#"{ "file": "/root/file.txt", "optional": true, "matches": "a|b|c" }"#,
            )
            .unwrap();

            assert_eq!(
                input,
                Input::File({
                    let mut inner = stub_file_input("/root/file.txt");
                    inner.optional = Some(true);
                    inner.content = Some(RegexSetting::new("a|b|c").unwrap());
                    inner
                })
            );
        }

        #[test]
        fn globs() {
            let input: Input = serde_json::from_str(r#""file.*""#).unwrap();

            assert_eq!(input, Input::Glob(stub_glob_input("file.*")));

            let input: Input = serde_json::from_str(r#"{ "glob": "file.*" }"#).unwrap();

            assert_eq!(input, Input::Glob(stub_glob_input("file.*")));

            let input: Input =
                serde_json::from_str(r#"{ "glob": "/dir/file.*", "cache": false }"#).unwrap();

            assert_eq!(
                input,
                Input::Glob({
                    let mut inner = stub_glob_input("/dir/file.*");
                    inner.cache = false;
                    inner
                })
            );
        }

        #[test]
        #[should_panic] // Swallowed by enum expecting message
        fn errors_for_parent_traversal() {
            let _: Input = serde_json::from_str(r#"{ "glob": "../../file.*" }"#).unwrap();
        }

        #[test]
        #[should_panic] // Swallowed by enum expecting message
        fn errors_for_parent_traversal_inner() {
            let _: Input = serde_json::from_str(r#"{ "glob": "dir/../../file.*" }"#).unwrap();
        }

        #[test]
        fn file_group() {
            let input: Input = serde_json::from_str(r#"{ "group": "sources" }"#).unwrap();

            assert_eq!(
                input,
                Input::FileGroup(FileGroupInput {
                    group: Id::raw("sources"),
                    ..Default::default()
                })
            );

            let input: Input =
                serde_json::from_str(r#"{ "group": "sources", "format": "files" }"#).unwrap();

            assert_eq!(
                input,
                Input::FileGroup(FileGroupInput {
                    group: Id::raw("sources"),
                    format: FileGroupInputFormat::Files,
                })
            );
        }

        #[test]
        fn project_sources() {
            let input: Input = serde_json::from_str(r#"{ "project": "app" }"#).unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "app".into(),
                    ..Default::default()
                })
            );

            let input: Input =
                serde_json::from_str(r#"{ "project": "app", "group": "sources" }"#).unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "app".into(),
                    group: Some(Id::raw("sources")),
                    ..Default::default()
                })
            );

            let input: Input = serde_json::from_str(
                r#"{ "project": "app", "group": "sources", "filter": ["src/**/*"] }"#,
            )
            .unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "app".into(),
                    group: Some(Id::raw("sources")),
                    filter: vec!["src/**/*".into()],
                })
            );
        }

        #[test]
        fn project_sources_all() {
            let input: Input = serde_json::from_str(r#"{ "project": "^" }"#).unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "^".into(),
                    ..Default::default()
                })
            );

            let input: Input =
                serde_json::from_str(r#"{ "project": "^", "group": "sources" }"#).unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "^".into(),
                    group: Some(Id::raw("sources")),
                    ..Default::default()
                })
            );

            let input: Input = serde_json::from_str(
                r#"{ "project": "^", "group": "sources", "filter": ["src/**/*"] }"#,
            )
            .unwrap();

            assert_eq!(
                input,
                Input::Project(ProjectInput {
                    project: "^".into(),
                    group: Some(Id::raw("sources")),
                    filter: vec!["src/**/*".into()],
                })
            );
        }
    }

    mod file {
        use super::*;

        #[test]
        fn project_relative() {
            let input = stub_file_input("project/file.txt");

            assert_eq!(input.file, "project/file.txt");
            assert_eq!(input.get_path(), "project/file.txt");
            assert!(!input.is_workspace_relative());

            let input = stub_file_input("./project/file.txt");

            assert_eq!(input.file, "project/file.txt");
            assert_eq!(input.get_path(), "project/file.txt");
            assert!(!input.is_workspace_relative());
        }

        #[test]
        fn workspace_relative() {
            let input = stub_file_input("/root/file.txt");

            assert_eq!(input.file, "/root/file.txt");
            assert_eq!(input.get_path(), "root/file.txt");
            assert!(input.is_workspace_relative());
        }

        #[test]
        fn supports_matches_field() {
            let input = stub_file_input("file.txt?matches=abc");

            assert_eq!(input.content.unwrap().as_str(), "abc");

            let input = stub_file_input("file.txt?match=abc");

            assert_eq!(input.content.unwrap().as_str(), "abc");

            let input = stub_file_input("file.txt?matches");

            assert!(input.content.is_none());
        }

        #[test]
        fn supports_optional_field() {
            let input = stub_file_input("file.txt?optional");

            assert!(input.optional.unwrap());

            let input = stub_file_input("file.txt?optional=true");

            assert!(input.optional.unwrap());

            let input = stub_file_input("file.txt?optional=false");

            assert!(!input.optional.unwrap());
        }

        #[test]
        #[should_panic(expected = "globs are not supported")]
        fn errors_for_glob() {
            stub_file_input("file.*");
        }

        #[test]
        #[should_panic(expected = "unsupported value for `optional`")]
        fn errors_invalid_optional_field() {
            stub_file_input("file.txt?optional=invalid");
        }

        #[test]
        #[should_panic(expected = "unknown file field `unknown`")]
        fn errors_unknown_field() {
            stub_file_input("file.txt?unknown");
        }
    }

    mod file_group {
        use super::*;

        #[test]
        fn id() {
            let input = FileGroupInput::from_uri(Uri::parse("group://sources").unwrap()).unwrap();

            assert_eq!(input.group, "sources");
        }

        #[test]
        fn supports_format_field() {
            let input =
                FileGroupInput::from_uri(Uri::parse("group://sources?format=dirs").unwrap())
                    .unwrap();

            assert_eq!(input.format, FileGroupInputFormat::Dirs);

            let input =
                FileGroupInput::from_uri(Uri::parse("group://sources?as=root").unwrap()).unwrap();

            assert_eq!(input.format, FileGroupInputFormat::Root);
        }

        #[test]
        #[should_panic(expected = "a file group identifier is required")]
        fn errors_no_id() {
            FileGroupInput::from_uri(Uri::parse("group://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format")]
        fn errors_invalid_id() {
            FileGroupInput::from_uri(Uri::parse("group://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Unknown enum variant")]
        fn errors_invalid_format_field() {
            FileGroupInput::from_uri(Uri::parse("group://id?format=unknown").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown file group field `unknown`")]
        fn errors_unknown_field() {
            FileGroupInput::from_uri(Uri::parse("group://id?unknown").unwrap()).unwrap();
        }
    }

    mod glob {
        use super::*;

        #[test]
        fn default_cache_enabled() {
            let input = stub_glob_input("file.*");

            assert!(input.cache);
        }

        #[test]
        fn project_relative() {
            let input = stub_glob_input("project/file.*");

            assert_eq!(input.glob, "project/file.*");
            assert_eq!(input.get_path(), "project/file.*");
            assert!(!input.is_workspace_relative());
            assert!(!input.is_negated());

            let input = stub_glob_input("./project/file.*");

            assert_eq!(input.glob, "project/file.*");
            assert_eq!(input.get_path(), "project/file.*");
            assert!(!input.is_workspace_relative());
            assert!(!input.is_negated());
        }

        #[test]
        fn project_relative_negated() {
            let input = stub_glob_input("!project/file.*");

            assert_eq!(input.glob, "!project/file.*");
            assert_eq!(input.get_path(), "!project/file.*");
            assert!(!input.is_workspace_relative());
            assert!(input.is_negated());

            let input = stub_glob_input("!./project/file.*");

            assert_eq!(input.glob, "!project/file.*");
            assert_eq!(input.get_path(), "!project/file.*");
            assert!(!input.is_workspace_relative());
            assert!(input.is_negated());

            let input = stub_glob_input("./!project/file.*");

            assert_eq!(input.glob, "!project/file.*");
            assert_eq!(input.get_path(), "!project/file.*");
            assert!(!input.is_workspace_relative());
            assert!(input.is_negated());
        }

        #[test]
        fn workspace_relative() {
            let input = stub_glob_input("/root/file.*");

            assert_eq!(input.glob, "/root/file.*");
            assert_eq!(input.get_path(), "root/file.*");
            assert!(input.is_workspace_relative());
            assert!(!input.is_negated());
        }

        #[test]
        fn workspace_relative_negated() {
            let input = stub_glob_input("!/root/file.*");

            assert_eq!(input.glob, "!/root/file.*");
            assert_eq!(input.get_path(), "!root/file.*");
            assert!(input.is_workspace_relative());
            assert!(input.is_negated());

            let input = stub_glob_input("/!root/file.*");

            assert_eq!(input.glob, "!/root/file.*");
            assert_eq!(input.get_path(), "!root/file.*");
            assert!(input.is_workspace_relative());
            assert!(input.is_negated());
        }

        #[test]
        fn supports_cache_field() {
            let input = stub_glob_input("glob://file.*?cache");

            assert!(input.cache);

            let input = stub_glob_input("glob://file.*?cache=true");

            assert!(input.cache);

            let input = stub_glob_input("glob://file.*?cache=false");

            assert!(!input.cache);
        }

        #[test]
        #[should_panic(expected = "unsupported value for `cache`")]
        fn errors_invalid_cache_field() {
            stub_glob_input("glob://file.*?cache=invalid");
        }

        #[test]
        #[should_panic(expected = "unknown glob field `unknown`")]
        fn errors_unknown_field() {
            stub_glob_input("glob://file.*?unknown");
        }
    }

    mod manifest_deps {
        use super::*;

        #[test]
        fn id() {
            let input =
                ManifestDepsInput::from_uri(Uri::parse("manifest://node").unwrap()).unwrap();

            assert_eq!(input.manifest, "node");
        }

        #[test]
        fn supports_deps_field() {
            for key in ["dep", "deps", "dependencies"] {
                let input = ManifestDepsInput::from_uri(
                    Uri::parse(format!("manifest://node?{key}").as_str()).unwrap(),
                )
                .unwrap();

                assert!(input.deps.is_empty());

                let input = ManifestDepsInput::from_uri(
                    Uri::parse(format!("manifest://node?{key}=a").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a"]);

                let input = ManifestDepsInput::from_uri(
                    Uri::parse(format!("manifest://node?{key}=a,b,c").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a", "b", "c"]);

                let input = ManifestDepsInput::from_uri(
                    Uri::parse(format!("manifest://node?{key}=a&{key}=b,c&{key}=d").as_str())
                        .unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a", "b", "c", "d"]);
            }
        }

        #[test]
        #[should_panic(expected = "a toolchain identifier is required")]
        fn errors_no_id() {
            ManifestDepsInput::from_uri(Uri::parse("manifest://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format")]
        fn errors_invalid_id() {
            ManifestDepsInput::from_uri(Uri::parse("manifest://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown manifest field `unknown`")]
        fn errors_unknown_field() {
            ManifestDepsInput::from_uri(Uri::parse("manifest://id?unknown").unwrap()).unwrap();
        }
    }

    mod external_project {
        use super::*;

        #[test]
        fn id() {
            let input = ProjectInput::from_uri(Uri::parse("project://app").unwrap()).unwrap();

            assert_eq!(input.project, "app");
        }

        #[test]
        fn supports_file_group_field() {
            for key in ["fileGroup", "group"] {
                let input = ProjectInput::from_uri(
                    Uri::parse(format!("project://app?{key}").as_str()).unwrap(),
                )
                .unwrap();

                assert!(input.group.is_none());

                let input = ProjectInput::from_uri(
                    Uri::parse(format!("project://app?{key}=a").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.group.unwrap(), "a");
            }
        }

        #[test]
        fn supports_filter_field() {
            let input =
                ProjectInput::from_uri(Uri::parse("project://app?filter").unwrap()).unwrap();

            assert!(input.filter.is_empty());

            let input =
                ProjectInput::from_uri(Uri::parse("project://app?filter=a&filter=b").unwrap())
                    .unwrap();

            assert_eq!(input.filter, ["a", "b"]);
        }

        #[test]
        #[should_panic(expected = "a project identifier is required")]
        fn errors_no_id() {
            ProjectInput::from_uri(Uri::parse("project://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid identifier format")]
        fn errors_invalid_id() {
            ProjectInput::from_uri(Uri::parse("project://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown project field `unknown`")]
        fn errors_unknown_field() {
            ProjectInput::from_uri(Uri::parse("project://id?unknown").unwrap()).unwrap();
        }
    }
}
