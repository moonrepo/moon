use moon_config::{GlobOutput, GlobPath, Output, test_utils::*};

mod output_shape {
    use super::*;

    mod parse_string {
        use super::*;

        #[test]
        fn converts_backward_slashes() {
            assert_eq!(
                Output::parse("some\\file.txt").unwrap(),
                Output::File(stub_file_output("some/file.txt"))
            );
        }

        #[test]
        fn token_funcs() {
            assert_eq!(
                Output::parse("@group(name)").unwrap(),
                Output::TokenFunc("@group(name)".into())
            );
            assert_eq!(
                Output::parse("@dirs(name)").unwrap(),
                Output::TokenFunc("@dirs(name)".into())
            );
            assert_eq!(
                Output::parse("@files(name)").unwrap(),
                Output::TokenFunc("@files(name)".into())
            );
            assert_eq!(
                Output::parse("@globs(name)").unwrap(),
                Output::TokenFunc("@globs(name)".into())
            );
            assert_eq!(
                Output::parse("@root(name)").unwrap(),
                Output::TokenFunc("@root(name)".into())
            );
        }

        #[test]
        fn token_vars() {
            assert_eq!(
                Output::parse("$workspaceRoot").unwrap(),
                Output::TokenVar("$workspaceRoot".into())
            );
            assert_eq!(
                Output::parse("$projectType").unwrap(),
                Output::TokenVar("$projectType".into())
            );
        }

        #[test]
        fn file_protocol() {
            let mut output = stub_file_output("file.txt");
            output.optional = Some(true);

            assert_eq!(
                Output::parse("file://file.txt?optional").unwrap(),
                Output::File(output)
            );

            let mut output = stub_file_output("/file.txt");
            output.optional = Some(false);

            assert_eq!(
                Output::parse("file:///file.txt?optional=false").unwrap(),
                Output::File(output)
            );
        }

        #[test]
        fn file_project_relative() {
            assert_eq!(
                Output::parse("file.rs").unwrap(),
                Output::File(stub_file_output("file.rs"))
            );
            assert_eq!(
                Output::parse("dir/file.rs").unwrap(),
                Output::File(stub_file_output("dir/file.rs"))
            );
            assert_eq!(
                Output::parse("./file.rs").unwrap(),
                Output::File(stub_file_output("file.rs"))
            );
            assert_eq!(
                Output::parse("././dir/file.rs").unwrap(),
                Output::File(stub_file_output("dir/file.rs"))
            );
        }

        #[test]
        fn file_project_relative_protocol() {
            assert_eq!(
                Output::parse("file://file.rs").unwrap(),
                Output::File(stub_file_output("file.rs"))
            );
            assert_eq!(
                Output::parse("file://dir/file.rs").unwrap(),
                Output::File(stub_file_output("dir/file.rs"))
            );
            assert_eq!(
                Output::parse("file://./file.rs").unwrap(),
                Output::File(stub_file_output("file.rs"))
            );
            assert_eq!(
                Output::parse("file://././dir/file.rs").unwrap(),
                Output::File(stub_file_output("dir/file.rs"))
            );
        }

        #[test]
        fn file_workspace_relative() {
            assert_eq!(
                Output::parse("/file.rs").unwrap(),
                Output::File(stub_file_output("/file.rs"))
            );
            assert_eq!(
                Output::parse("/dir/file.rs").unwrap(),
                Output::File(stub_file_output("/dir/file.rs"))
            );

            // With tokens
            assert_eq!(
                Output::parse("/.cache/$projectSource").unwrap(),
                Output::File(stub_file_output("/.cache/$projectSource"))
            );
        }

        #[test]
        fn file_workspace_relative_protocol() {
            assert_eq!(
                Output::parse("file:///file.rs").unwrap(),
                Output::File(stub_file_output("/file.rs"))
            );
            assert_eq!(
                Output::parse("file:///dir/file.rs").unwrap(),
                Output::File(stub_file_output("/dir/file.rs"))
            );

            // With tokens
            assert_eq!(
                Output::parse("file:///.cache/$projectSource").unwrap(),
                Output::File(stub_file_output("/.cache/$projectSource"))
            );
        }

        #[test]
        fn glob_protocol() {
            assert_eq!(
                Output::parse("glob://file.*").unwrap(),
                Output::Glob(stub_glob_output("file.*"))
            );

            assert_eq!(
                Output::parse("glob:///file.*").unwrap(),
                Output::Glob(stub_glob_output("/file.*"))
            );

            let mut output = stub_glob_output("/file.*");
            output.optional = Some(true);

            assert_eq!(
                Output::parse("glob:///file.*?optional").unwrap(),
                Output::Glob(output)
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
                assert_eq!(
                    Output::Glob(GlobOutput {
                        glob: GlobPath(pat.into()),
                        optional: None,
                    }),
                    Output::Glob(stub_glob_output(pat))
                );
            }
        }

        #[test]
        fn glob_project_relative() {
            assert_eq!(
                Output::parse("!file.*").unwrap(),
                Output::Glob(stub_glob_output("!file.*"))
            );
            assert_eq!(
                Output::parse("dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("dir/**/*"))
            );
            assert_eq!(
                Output::parse("./dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("dir/**/*"))
            );

            // With tokens
            assert_eq!(
                Output::parse("$projectSource/**/*").unwrap(),
                Output::Glob(stub_glob_output("$projectSource/**/*"))
            );
        }

        #[test]
        fn glob_project_relative_protocol() {
            assert_eq!(
                Output::parse("glob://!file.*").unwrap(),
                Output::Glob(stub_glob_output("!file.*"))
            );
            assert_eq!(
                Output::parse("glob://dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("dir/**/*"))
            );
            assert_eq!(
                Output::parse("glob://./dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("dir/**/*"))
            );

            // With tokens
            assert_eq!(
                Output::parse("glob://$projectSource/**/*").unwrap(),
                Output::Glob(stub_glob_output("$projectSource/**/*"))
            );
        }

        #[test]
        fn glob_workspace_relative() {
            assert_eq!(
                Output::parse("/!file.*").unwrap(),
                Output::Glob(stub_glob_output("!/file.*"))
            );
            assert_eq!(
                Output::parse("!/file.*").unwrap(),
                Output::Glob(stub_glob_output("!/file.*"))
            );
            assert_eq!(
                Output::parse("/dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("/dir/**/*"))
            );
        }

        #[test]
        fn glob_workspace_relative_protocol() {
            assert_eq!(
                Output::parse("glob:///!file.*").unwrap(),
                Output::Glob(stub_glob_output("!/file.*"))
            );
            assert_eq!(
                Output::parse("glob://!/file.*").unwrap(),
                Output::Glob(stub_glob_output("!/file.*"))
            );
            assert_eq!(
                Output::parse("glob:///dir/**/*").unwrap(),
                Output::Glob(stub_glob_output("/dir/**/*"))
            );
        }

        #[test]
        #[should_panic(expected = "environment variable globs are not supported")]
        fn errors_for_env_globs() {
            Output::parse("$VAR_*").unwrap();
        }

        #[test]
        #[should_panic(expected = "output protocol `unknown://` is not supported")]
        fn errors_for_unknown_protocol() {
            Output::parse("unknown://test").unwrap();
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_for_parent_traversal() {
            Output::parse("../../file.txt").unwrap();
        }

        #[test]
        #[should_panic(expected = "parent directory traversal (..) is not supported")]
        fn errors_for_parent_traversal_inner() {
            Output::parse("dir/../../file.txt").unwrap();
        }
    }

    mod parse_object {
        use super::*;

        #[test]
        fn files() {
            let output: Output = serde_json::from_str(r#""file.txt""#).unwrap();

            assert_eq!(output, Output::File(stub_file_output("file.txt")));

            let output: Output = serde_json::from_str(r#"{ "file": "file.txt" }"#).unwrap();

            assert_eq!(output, Output::File(stub_file_output("file.txt")));

            let output: Output =
                serde_json::from_str(r#"{ "file": "dir/file.txt", "optional": true }"#).unwrap();

            assert_eq!(
                output,
                Output::File({
                    let mut inner = stub_file_output("dir/file.txt");
                    inner.optional = Some(true);
                    inner
                })
            );
        }

        #[test]
        fn globs() {
            let output: Output = serde_json::from_str(r#""file.*""#).unwrap();

            assert_eq!(output, Output::Glob(stub_glob_output("file.*")));

            let output: Output = serde_json::from_str(r#"{ "glob": "file.*" }"#).unwrap();

            assert_eq!(output, Output::Glob(stub_glob_output("file.*")));
        }

        #[test]
        #[should_panic] // Swallowed by enum expecting message
        fn errors_for_parent_traversal() {
            let _: Output = serde_json::from_str(r#"{ "glob": "../../file.*" }"#).unwrap();
        }

        #[test]
        #[should_panic] // Swallowed by enum expecting message
        fn errors_for_parent_traversal_inner() {
            let _: Output = serde_json::from_str(r#"{ "glob": "dir/../../file.*" }"#).unwrap();
        }
    }

    mod file {
        use super::*;

        #[test]
        fn project_relative() {
            let output = stub_file_output("project/file.txt");

            assert_eq!(output.file, "project/file.txt");
            assert_eq!(output.get_path(), "project/file.txt");
            assert!(!output.is_workspace_relative());

            let output = stub_file_output("./project/file.txt");

            assert_eq!(output.file, "project/file.txt");
            assert_eq!(output.get_path(), "project/file.txt");
            assert!(!output.is_workspace_relative());
        }

        #[test]
        fn workspace_relative() {
            let output = stub_file_output("/root/file.txt");

            assert_eq!(output.file, "/root/file.txt");
            assert_eq!(output.get_path(), "root/file.txt");
            assert!(output.is_workspace_relative());
        }

        #[test]
        fn supports_optional_field() {
            let output = stub_file_output("file.txt?optional");

            assert!(output.optional.unwrap());

            let output = stub_file_output("file.txt?optional=true");

            assert!(output.optional.unwrap());

            let output = stub_file_output("file.txt?optional=false");

            assert!(!output.optional.unwrap());
        }

        #[test]
        #[should_panic(expected = "globs are not supported")]
        fn errors_for_glob() {
            stub_file_output("file.*");
        }

        #[test]
        #[should_panic(expected = "unsupported value for `optional`")]
        fn errors_invalid_optional_field() {
            stub_file_output("file.txt?optional=invalid");
        }

        #[test]
        #[should_panic(expected = "unknown file field `unknown`")]
        fn errors_unknown_field() {
            stub_file_output("file.txt?unknown");
        }
    }

    mod glob {
        use super::*;

        #[test]
        fn project_relative() {
            let output = stub_glob_output("project/file.*");

            assert_eq!(output.glob, "project/file.*");
            assert_eq!(output.get_path(), "project/file.*");
            assert!(!output.is_workspace_relative());
            assert!(!output.is_negated());

            let output = stub_glob_output("./project/file.*");

            assert_eq!(output.glob, "project/file.*");
            assert_eq!(output.get_path(), "project/file.*");
            assert!(!output.is_workspace_relative());
            assert!(!output.is_negated());
        }

        #[test]
        fn project_relative_negated() {
            let output = stub_glob_output("!project/file.*");

            assert_eq!(output.glob, "!project/file.*");
            assert_eq!(output.get_path(), "!project/file.*");
            assert!(!output.is_workspace_relative());
            assert!(output.is_negated());

            let output = stub_glob_output("!./project/file.*");

            assert_eq!(output.glob, "!project/file.*");
            assert_eq!(output.get_path(), "!project/file.*");
            assert!(!output.is_workspace_relative());
            assert!(output.is_negated());

            let output = stub_glob_output("./!project/file.*");

            assert_eq!(output.glob, "!project/file.*");
            assert_eq!(output.get_path(), "!project/file.*");
            assert!(!output.is_workspace_relative());
            assert!(output.is_negated());
        }

        #[test]
        fn workspace_relative() {
            let output = stub_glob_output("/root/file.*");

            assert_eq!(output.glob, "/root/file.*");
            assert_eq!(output.get_path(), "root/file.*");
            assert!(output.is_workspace_relative());
            assert!(!output.is_negated());
        }

        #[test]
        fn workspace_relative_negated() {
            let output = stub_glob_output("!/root/file.*");

            assert_eq!(output.glob, "!/root/file.*");
            assert_eq!(output.get_path(), "!root/file.*");
            assert!(output.is_workspace_relative());
            assert!(output.is_negated());

            let output = stub_glob_output("/!root/file.*");

            assert_eq!(output.glob, "!/root/file.*");
            assert_eq!(output.get_path(), "!root/file.*");
            assert!(output.is_workspace_relative());
            assert!(output.is_negated());
        }

        #[test]
        #[should_panic(expected = "unknown glob field `cache`")]
        fn errors_invalid_cache_field() {
            stub_glob_output("glob://file.*?cache=invalid");
        }

        #[test]
        #[should_panic(expected = "unknown glob field `unknown`")]
        fn errors_unknown_field() {
            stub_glob_output("glob://file.*?unknown");
        }
    }
}
