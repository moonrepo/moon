use moon_config::{
    FileGroupInput, FileGroupInputFormat, FileInput, GlobInput, ManifestDepsInput,
    ProjectSourcesInput,
};
use url::Url;

mod input_shape {
    use super::*;

    mod file {
        use super::*;

        #[test]
        fn project_relative() {
            let input =
                FileInput::from_uri(Url::parse("file://project/file.txt").unwrap()).unwrap();

            assert_eq!(input.file, "project/file.txt");

            let input =
                FileInput::from_uri(Url::parse("file://./project/file.txt").unwrap()).unwrap();

            assert_eq!(input.file, "project/file.txt");
        }

        #[test]
        fn workspace_relative() {
            let input = FileInput::from_uri(Url::parse("file:///root/file.txt").unwrap()).unwrap();

            assert_eq!(input.file, "/root/file.txt");
        }

        #[test]
        fn supports_matches_field() {
            let input =
                FileInput::from_uri(Url::parse("file://file.txt?matches=abc").unwrap()).unwrap();

            assert_eq!(input.matches.unwrap(), "abc");

            let input =
                FileInput::from_uri(Url::parse("file://file.txt?match=abc").unwrap()).unwrap();

            assert_eq!(input.matches.unwrap(), "abc");

            let input =
                FileInput::from_uri(Url::parse("file://file.txt?matches").unwrap()).unwrap();

            assert!(input.matches.is_none());
        }

        #[test]
        fn supports_optional_field() {
            let input =
                FileInput::from_uri(Url::parse("file://file.txt?optional").unwrap()).unwrap();

            assert!(input.optional);

            let input =
                FileInput::from_uri(Url::parse("file://file.txt?optional=true").unwrap()).unwrap();

            assert!(input.optional);

            let input =
                FileInput::from_uri(Url::parse("file://file.txt?optional=false").unwrap()).unwrap();

            assert!(!input.optional);
        }

        #[test]
        #[should_panic(expected = "globs are not supported")]
        fn errors_for_glob() {
            FileInput::from_uri(Url::parse("file://file.*").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unsupported value for `optional`")]
        fn errors_invalid_optional_field() {
            FileInput::from_uri(Url::parse("file://file.txt?optional=invalid").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_field() {
            FileInput::from_uri(Url::parse("file://file.txt?unknown").unwrap()).unwrap();
        }
    }

    mod file_group {
        use super::*;

        #[test]
        fn id() {
            let input = FileGroupInput::from_uri(Url::parse("group://sources").unwrap()).unwrap();

            assert_eq!(input.group, "sources");
        }

        #[test]
        fn supports_format_field() {
            let input =
                FileGroupInput::from_uri(Url::parse("group://sources?format=dirs").unwrap())
                    .unwrap();

            assert_eq!(input.format, FileGroupInputFormat::Dirs);

            let input =
                FileGroupInput::from_uri(Url::parse("group://sources?as=root").unwrap()).unwrap();

            assert_eq!(input.format, FileGroupInputFormat::Root);
        }

        #[test]
        #[should_panic(expected = "a file group identifier is required")]
        fn errors_no_id() {
            FileGroupInput::from_uri(Url::parse("group://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid format")]
        fn errors_invalid_id() {
            FileGroupInput::from_uri(Url::parse("group://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Unknown enum variant")]
        fn errors_invalid_format_field() {
            FileGroupInput::from_uri(Url::parse("group://id?format=unknown").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_field() {
            FileGroupInput::from_uri(Url::parse("group://id?unknown").unwrap()).unwrap();
        }
    }

    mod glob {
        use super::*;

        #[test]
        fn default_cache_enabled() {
            let input = GlobInput::from_uri(Url::parse("glob://file.*").unwrap()).unwrap();

            assert!(input.cache);
        }

        #[test]
        fn project_relative() {
            let input = GlobInput::from_uri(Url::parse("glob://project/file.*").unwrap()).unwrap();

            assert_eq!(input.glob, "project/file.*");

            let input =
                GlobInput::from_uri(Url::parse("glob://./project/file.*").unwrap()).unwrap();

            assert_eq!(input.glob, "project/file.*");
        }

        #[test]
        fn workspace_relative() {
            let input = GlobInput::from_uri(Url::parse("glob:///root/file.*").unwrap()).unwrap();

            assert_eq!(input.glob, "/root/file.*");
        }

        #[test]
        fn supports_optional_field() {
            let input = GlobInput::from_uri(Url::parse("glob://file.*?cache").unwrap()).unwrap();

            assert!(input.cache);

            let input =
                GlobInput::from_uri(Url::parse("glob://file.*?cache=true").unwrap()).unwrap();

            assert!(input.cache);

            let input =
                GlobInput::from_uri(Url::parse("glob://file.*?cache=false").unwrap()).unwrap();

            assert!(!input.cache);
        }

        #[test]
        #[should_panic(expected = "unsupported value for `cache`")]
        fn errors_invalid_cache_field() {
            GlobInput::from_uri(Url::parse("glob://file.*?cache=invalid").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_field() {
            GlobInput::from_uri(Url::parse("glob://file.*?unknown").unwrap()).unwrap();
        }
    }

    mod manifest_deps {
        use super::*;

        #[test]
        fn id() {
            let input =
                ManifestDepsInput::from_uri(Url::parse("manifest://node").unwrap()).unwrap();

            assert_eq!(input.manifest, "node");
        }

        #[test]
        fn supports_deps_field() {
            for key in ["dep", "deps", "dependencies"] {
                let input = ManifestDepsInput::from_uri(
                    Url::parse(format!("manifest://node?{key}").as_str()).unwrap(),
                )
                .unwrap();

                assert!(input.deps.is_empty());

                let input = ManifestDepsInput::from_uri(
                    Url::parse(format!("manifest://node?{key}=a").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a"]);

                let input = ManifestDepsInput::from_uri(
                    Url::parse(format!("manifest://node?{key}=a,b,c").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a", "b", "c"]);

                let input = ManifestDepsInput::from_uri(
                    Url::parse(format!("manifest://node?{key}=a&{key}=b,c&{key}=d").as_str())
                        .unwrap(),
                )
                .unwrap();

                assert_eq!(input.deps, ["a", "b", "c", "d"]);
            }
        }

        #[test]
        #[should_panic(expected = "a toolchain identifier is required")]
        fn errors_no_id() {
            ManifestDepsInput::from_uri(Url::parse("manifest://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid format")]
        fn errors_invalid_id() {
            ManifestDepsInput::from_uri(Url::parse("manifest://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_field() {
            ManifestDepsInput::from_uri(Url::parse("manifest://id?unknown").unwrap()).unwrap();
        }
    }

    mod project_srcs {
        use super::*;

        #[test]
        fn id() {
            let input =
                ProjectSourcesInput::from_uri(Url::parse("project://app").unwrap()).unwrap();

            assert_eq!(input.project, "app");
        }

        #[test]
        fn supports_file_group_field() {
            for key in ["fileGroup", "file-group", "group"] {
                let input = ProjectSourcesInput::from_uri(
                    Url::parse(format!("project://app?{key}").as_str()).unwrap(),
                )
                .unwrap();

                assert!(input.group.is_none());

                let input = ProjectSourcesInput::from_uri(
                    Url::parse(format!("project://app?{key}=a").as_str()).unwrap(),
                )
                .unwrap();

                assert_eq!(input.group.unwrap(), "a");
            }
        }

        #[test]
        fn supports_filter_field() {
            let input =
                ProjectSourcesInput::from_uri(Url::parse("project://app?filter").unwrap()).unwrap();

            assert!(input.filter.is_empty());

            let input = ProjectSourcesInput::from_uri(
                Url::parse("project://app?filter=a&filter=b").unwrap(),
            )
            .unwrap();

            assert_eq!(input.filter, ["a", "b"]);
        }

        #[test]
        #[should_panic(expected = "a project identifier is required")]
        fn errors_no_id() {
            ProjectSourcesInput::from_uri(Url::parse("project://").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "Invalid format")]
        fn errors_invalid_id() {
            ProjectSourcesInput::from_uri(Url::parse("project://@&n3k(").unwrap()).unwrap();
        }

        #[test]
        #[should_panic(expected = "unknown field `unknown`")]
        fn errors_unknown_field() {
            ProjectSourcesInput::from_uri(Url::parse("project://id?unknown").unwrap()).unwrap();
        }
    }
}
