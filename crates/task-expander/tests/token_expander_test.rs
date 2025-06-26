mod utils;

use moon_common::path::{self, WorkspaceRelativePathBuf};
use moon_config::{InputPath, LanguageType, LayerType, OutputPath};
use moon_env_var::GlobalEnvBag;
use moon_task_expander::{ExpandedResult, TokenExpander};
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_sandbox::{create_empty_sandbox, create_sandbox, predicates::prelude::*};
use std::borrow::Cow;
use std::env;
use utils::{create_context, create_project, create_task};

mod token_expander {
    use super::*;

    #[test]
    #[should_panic(expected = "Unknown token @unknown(id).")]
    fn errors_for_unknown_token_func() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@unknown(id)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Unknown file group unknown used in token @files(unknown).")]
    fn errors_for_unknown_file_group() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@files(unknown)").unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Token @in(str) in task project:task received an invalid type for index"
    )]
    fn errors_for_invalid_in_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@in(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Input index 10 does not exist for token @in(10)")]
    fn errors_for_invalid_in_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@in(10)").unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Token @out(str) in task project:task received an invalid type for index"
    )]
    fn errors_for_invalid_out_index_type() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@out(str)").unwrap();
    }

    #[test]
    #[should_panic(expected = "Output index 10 does not exist for token @out(10)")]
    fn errors_for_invalid_out_index() {
        let sandbox = create_empty_sandbox();
        let project = create_project(sandbox.path());
        let task = create_task();
        let context = create_context(sandbox.path());
        let expander = TokenExpander::new(&project, &context);

        expander.replace_function(&task, "@out(10)").unwrap();
    }

    mod funcs {
        use super::*;

        #[test]
        fn in_can_ref_other_token_funcs() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();
            task.inputs.push(InputPath::TokenFunc("@globs(all)".into()));

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            let result = expander.replace_function(&task, "@in(0)").unwrap();

            assert_eq!(
                result.globs,
                ["project/source/*.md", "project/source/**/*.json"]
            );
        }

        #[test]
        #[should_panic(expected = "Unknown file group unknown")]
        fn errors_if_in_refs_invalid_group() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();
            task.inputs
                .push(InputPath::TokenFunc("@globs(unknown)".into()));

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            expander.replace_function(&task, "@in(0)").unwrap();
        }

        #[test]
        fn out_can_ref_other_token_funcs() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();
            task.outputs
                .push(OutputPath::TokenFunc("@globs(all)".into()));

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            let result = expander.replace_function(&task, "@out(0)").unwrap();

            assert_eq!(
                result.globs,
                ["project/source/*.md", "project/source/**/*.json"]
            );
        }

        #[test]
        #[should_panic(expected = "Unknown file group unknown")]
        fn errors_if_out_refs_invalid_group() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();
            task.outputs
                .push(OutputPath::TokenFunc("@globs(unknown)".into()));

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            expander.replace_function(&task, "@out(0)").unwrap();
        }

        #[test]
        fn meta_refs_native_metadata() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            let metadata = project.config.project.get_or_insert(Default::default());

            metadata.name = Some("name".into());
            metadata.description = "description".into();
            metadata.channel = Some("#channel".into());
            metadata.owner = Some("owner".into());
            metadata.maintainers.push("user1".into());
            metadata.maintainers.push("user2".into());

            let task = create_task();
            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            let get_value = |token: &str| {
                expander
                    .replace_function(&task, token)
                    .unwrap()
                    .value
                    .unwrap()
            };

            assert_eq!(get_value("@meta(name)"), "name");
            assert_eq!(get_value("@meta(description)"), "description");
            assert_eq!(get_value("@meta(channel)"), "#channel");
            assert_eq!(get_value("@meta(owner)"), "owner");
            assert_eq!(get_value("@meta(maintainers)"), "user1,user2");
        }

        #[test]
        fn meta_refs_custom_metadata() {
            use starbase_utils::json::JsonValue;

            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            let metadata = project.config.project.get_or_insert(Default::default());

            metadata.name = Some("name".into());
            metadata
                .metadata
                .insert("name".into(), JsonValue::String("custom-name".into()));
            metadata
                .metadata
                .insert("string".into(), JsonValue::String("custom".into()));
            metadata.metadata.insert(
                "list".into(),
                JsonValue::Array(vec![
                    JsonValue::String("item".into()),
                    JsonValue::Bool(true),
                ]),
            );

            let task = create_task();
            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            let get_value = |token: &str| {
                expander
                    .replace_function(&task, token)
                    .unwrap()
                    .value
                    .unwrap()
            };

            assert_eq!(get_value("@meta(name)"), "name");
            assert_eq!(get_value("@meta(string)"), "\"custom\"");
            assert_eq!(get_value("@meta(list)"), "[\"item\",true]");
        }
    }

    mod vars {
        use super::*;
        use env::consts;

        #[test]
        fn replaces_variables() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.layer = LayerType::Library;
            project.language = LanguageType::JavaScript;

            let task = create_task();

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language"))
                    .unwrap(),
                "javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$project"))
                    .unwrap(),
                "project"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectAlias"))
                    .unwrap(),
                ""
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectSource"))
                    .unwrap(),
                "project/source"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectRoot"))
                    .unwrap(),
                path::to_string(&project.root).unwrap()
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$projectType"))
                    .unwrap(),
                "library"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$target"))
                    .unwrap(),
                "project:task"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$task"))
                    .unwrap(),
                "task"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$taskToolchain"))
                    .unwrap(),
                "system"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$taskType"))
                    .unwrap(),
                "test"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$workingDir"))
                    .unwrap(),
                path::to_string(sandbox.path()).unwrap()
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$workspaceRoot"))
                    .unwrap(),
                path::to_string(sandbox.path()).unwrap()
            );

            assert!(
                predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}")
                    .unwrap()
                    .eval(
                        &expander
                            .replace_variable(&task, Cow::Borrowed("$date"))
                            .unwrap()
                    )
            );

            assert!(
                predicate::str::is_match("[0-9]{2}:[0-9]{2}:[0-9]{2}")
                    .unwrap()
                    .eval(
                        &expander
                            .replace_variable(&task, Cow::Borrowed("$time"))
                            .unwrap()
                    )
            );

            assert!(
                predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}:[0-9]{2}:[0-9]{2}")
                    .unwrap()
                    .eval(
                        &expander
                            .replace_variable(&task, Cow::Borrowed("$datetime"))
                            .unwrap()
                    )
            );

            assert!(
                predicate::str::is_match("[0-9]{10}").unwrap().eval(
                    &expander
                        .replace_variable(&task, Cow::Borrowed("$timestamp"))
                        .unwrap()
                )
            );

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$arch"))
                    .unwrap(),
                consts::ARCH
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$os"))
                    .unwrap(),
                consts::OS
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$osFamily"))
                    .unwrap(),
                consts::FAMILY
            );

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$vcsBranch"))
                    .unwrap(),
                "master"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$vcsRepository"))
                    .unwrap(),
                "moonrepo/moon"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$vcsRevision"))
                    .unwrap(),
                "abcd1234"
            );
        }

        #[test]
        fn replaces_variable_at_different_positions() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.language = LanguageType::JavaScript;
            let task = create_task();

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language"))
                    .unwrap(),
                "javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$language/before"))
                    .unwrap(),
                "javascript/before"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("after/$language"))
                    .unwrap(),
                "after/javascript"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("in/$language/between"))
                    .unwrap(),
                "in/javascript/between"
            );
            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("partof$languagestring"))
                    .unwrap(),
                "partofjavascriptstring"
            );
        }

        #[test]
        fn doesnt_clobber_same_name_variables() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());
            project.language = LanguageType::JavaScript;
            let task = create_task();

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander
                    .replace_variables(&task, "$project $projectStack $projectType")
                    .unwrap(),
                "project unknown unknown"
            );
            assert_eq!(
                expander
                    .replace_variables(&task, "$projectStack $project $projectType")
                    .unwrap(),
                "unknown project unknown"
            );
            assert_eq!(
                expander
                    .replace_variables(&task, "$projectType $projectStack $project")
                    .unwrap(),
                "unknown unknown project"
            );
        }

        #[test]
        fn keeps_unknown_var_as_is() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let task = create_task();

            let context = create_context(sandbox.path());
            let expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander
                    .replace_variable(&task, Cow::Borrowed("$unknown"))
                    .unwrap(),
                "$unknown"
            );
        }
    }

    mod command {
        use super::*;

        #[test]
        #[should_panic(expected = "Token @files(sources) in task project:task cannot be used")]
        fn errors_for_func() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "@files(sources)".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_command(&mut task).unwrap();
        }

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "bin".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_command(&mut task).unwrap(), "bin");
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_command(&mut task).unwrap(), "project/bin");
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$project/bin/$task".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_command(&mut task).unwrap(),
                "project/bin/task"
            );
        }

        #[test]
        fn supports_meta_func() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            project
                .config
                .project
                .get_or_insert(Default::default())
                .name = Some("name".into());

            let mut task = create_task();

            task.command = "@meta(name)".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_command(&mut task).unwrap(), "name");
        }

        #[test]
        fn inherits_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$FOO".into();

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_command(&mut task).unwrap(), "$FOO");
            assert_eq!(task.input_env, FxHashSet::from_iter(["FOO".into()]));
        }

        #[test]
        fn doesnt_inherit_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$FOO".into();
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_command(&mut task).unwrap(), "$FOO");
            assert!(task.input_env.is_empty());
        }

        #[test]
        fn doesnt_inherit_inputs_from_env_var_that_is_blacklisted() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.command = "$CI_HEAD/$COMMIT_SHA".into();
            task.options.infer_inputs = true;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_command(&mut task).unwrap(),
                "$CI_HEAD/$COMMIT_SHA"
            );
            assert!(task.input_env.is_empty());
        }
    }

    mod args {
        use super::*;

        #[test]
        fn supports_meta_func() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            project
                .config
                .project
                .get_or_insert(Default::default())
                .name = Some("name".into());

            let mut task = create_task();

            task.args.push("@meta(name)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_args(&mut task).unwrap(), vec!["name"]);
        }

        #[test]
        fn inherits_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.args.push("$FOO".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_args(&mut task).unwrap(), vec!["$FOO"]);
            assert_eq!(task.input_env, FxHashSet::from_iter(["FOO".into()]));
        }

        #[test]
        fn doesnt_inherit_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.args.push("$FOO".into());
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_args(&mut task).unwrap(), vec!["$FOO"]);
            assert!(task.input_env.is_empty());
        }

        #[test]
        fn inherits_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.args.push("@group(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_args(&mut task).unwrap(),
                vec!["./config.yml", "./dir/subdir", "./*.md", "./**/*.json"]
            );
            assert_eq!(
                task.input_files,
                FxHashSet::from_iter([
                    "project/source/config.yml".into(),
                    "project/source/dir/subdir".into()
                ])
            );
            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([
                    "project/source/**/*.json".into(),
                    "project/source/*.md".into()
                ])
            );
        }

        #[test]
        fn doesnt_inherit_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.args.push("@group(all)".into());
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_args(&mut task).unwrap(),
                vec!["./config.yml", "./dir/subdir", "./*.md", "./**/*.json"]
            );
            assert!(task.input_files.is_empty());
            assert!(task.input_globs.is_empty());
        }

        #[test]
        fn can_use_env_and_token_vars_together() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("FOO".into(), "bar".into());
            task.args.push("$FOO/$project".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_args(&mut task).unwrap(),
                vec!["$FOO/project"]
            );
        }
    }

    mod envs {
        use super::*;

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("KEY".into(), "value".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("KEY".into(), "value".into())])
            );
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("VAR".into(), "$project-prod".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("VAR".into(), "project-prod".into())])
            );
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env
                .insert("VARS".into(), "$project-debug-$task".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("VARS".into(), "project-debug-task".into())])
            );
        }

        #[test]
        fn inherits_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GROUP".into(), "@group(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([(
                    "GROUP".into(),
                    "./config.yml,./dir/subdir,./*.md,./**/*.json".into()
                )])
            );
            assert_eq!(
                task.input_files,
                FxHashSet::from_iter([
                    "project/source/config.yml".into(),
                    "project/source/dir/subdir".into()
                ])
            );
            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([
                    "project/source/**/*.json".into(),
                    "project/source/*.md".into()
                ])
            );
        }

        #[test]
        fn doesnt_inherit_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GROUP".into(), "@group(all)".into());
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([(
                    "GROUP".into(),
                    "./config.yml,./dir/subdir,./*.md,./**/*.json".into()
                )])
            );
            assert!(task.input_files.is_empty());
            assert!(task.input_globs.is_empty());
        }

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GROUP".into(), "@group(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([(
                    "GROUP".into(),
                    "./config.yml,./dir/subdir,./*.md,./**/*.json".into()
                )])
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("DIRS".into(), "@dirs(dirs)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("DIRS".into(), "./dir/subdir,./other".into())])
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("FILES".into(), "@files(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([(
                    "FILES".into(),
                    "./config.yml,./dir/subdir/nested.json,./docs.md,./other/file.json".into()
                )])
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("GLOBS".into(), "@globs(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("GLOBS".into(), "./*.md,./**/*.json".into())])
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("ROOT".into(), "@root(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("ROOT".into(), "./dir/subdir".into())])
            );
        }

        #[test]
        fn supports_meta_func() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            project
                .config
                .project
                .get_or_insert(Default::default())
                .name = Some("name".into());

            let mut task = create_task();

            task.env.insert("ROOT".into(), "@meta(name)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_env(&mut task).unwrap(),
                FxHashMap::from_iter([("ROOT".into(), "name".into())])
            );
        }

        #[test]
        #[should_panic(
            expected = "Token @in(0) in task project:task cannot be used within task env."
        )]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("IN".into(), "@in(0)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_env(&mut task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @out(0) in task project:task cannot be used within task env."
        )]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("OUT".into(), "@out(0)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_env(&mut task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @envs(envs) in task project:task cannot be used within task env."
        )]
        fn errors_for_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("OUT".into(), "@envs(envs)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_env(&mut task).unwrap();
        }
    }

    mod inputs {
        use super::*;

        #[test]
        fn supports_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::EnvVar("FOO_BAR".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    env: vec!["FOO_BAR".into()],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_env_var_glob() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::EnvVarGlob("FOO_*".into())];

            let bag = GlobalEnvBag::instance();
            bag.set("FOO_ONE", "1");
            bag.set("FOO_TWO", "2");
            bag.set("FOO_THREE", "3");
            bag.set("BAR_ONE", "1");

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            let mut result = expander.expand_inputs(&task).unwrap();
            result.env.sort();

            assert_eq!(
                result,
                ExpandedResult {
                    env: vec!["FOO_ONE".into(), "FOO_THREE".into(), "FOO_TWO".into()],
                    ..ExpandedResult::default()
                }
            );

            bag.remove("FOO_ONE");
            bag.remove("FOO_TWO");
            bag.remove("FOO_THREE");
            bag.remove("BAR_ONE");
        }

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@group(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@dirs(dirs)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir"),
                        WorkspaceRelativePathBuf::from("project/source/other"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@files(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir/nested.json"),
                        WorkspaceRelativePathBuf::from("project/source/docs.md"),
                        WorkspaceRelativePathBuf::from("project/source/other/file.json"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@globs(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@root(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![WorkspaceRelativePathBuf::from("project/source/dir/subdir")],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@envs(envs)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    env: vec!["FOO_BAR".into()],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        #[should_panic(
            expected = "Token @in(0) in task project:task cannot be used within task inputs."
        )]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@in(0)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_inputs(&task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @out(0) in task project:task cannot be used within task inputs."
        )]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@out(0)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_inputs(&task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @meta(name) in task project:task cannot be used within task inputs."
        )]
        fn errors_for_meta_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![InputPath::TokenFunc("@meta(name)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_inputs(&task).unwrap();
        }

        #[test]
        fn supports_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![
                InputPath::TokenVar("$target".into()),
                InputPath::TokenVar("$taskPlatform".into()),
            ];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/project:task"),
                        WorkspaceRelativePathBuf::from("project/source/system"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_vars_in_paths() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.inputs = vec![
                InputPath::ProjectFile("$task/file.txt".into()),
                InputPath::ProjectGlob("$task/files/**/*".into()),
                InputPath::WorkspaceFile("cache/$target/file.txt".into()),
                InputPath::WorkspaceGlob("cache/$target/files/**/*".into()),
            ];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_inputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/project:task/file.txt"),
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/project:task/files/**/*"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }
    }

    mod outputs {
        use super::*;

        #[test]
        fn supports_group_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@group(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir")
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_dirs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@dirs(dirs)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir"),
                        WorkspaceRelativePathBuf::from("project/source/other"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_files_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@files(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/config.yml"),
                        WorkspaceRelativePathBuf::from("project/source/dir/subdir/nested.json"),
                        WorkspaceRelativePathBuf::from("project/source/docs.md"),
                        WorkspaceRelativePathBuf::from("project/source/other/file.json"),
                    ],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_globs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@globs(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/*.md"),
                        WorkspaceRelativePathBuf::from("project/source/**/*.json"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn supports_root_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@root(all)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![WorkspaceRelativePathBuf::from("project/source/dir/subdir")],
                    globs: vec![],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        #[should_panic(
            expected = "Token @in(0) in task project:task cannot be used within task outputs."
        )]
        fn errors_for_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@in(0)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @out(0) in task project:task cannot be used within task outputs."
        )]
        fn errors_for_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@out(0)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @meta(name) in task project:task cannot be used within task outputs."
        )]
        fn errors_for_meta_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@meta(name)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        #[should_panic(
            expected = "Token @envs(envs) in task project:task cannot be used within task outputs."
        )]
        fn errors_for_envs_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![OutputPath::TokenFunc("@envs(envs)".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            expander.expand_outputs(&task).unwrap();
        }

        #[test]
        fn converts_variables() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.outputs = vec![
                OutputPath::ProjectFile("$task/file.txt".into()),
                OutputPath::ProjectGlob("$task/files/**/*".into()),
                OutputPath::WorkspaceFile("cache/$target/file.txt".into()),
                OutputPath::WorkspaceGlob("cache/$target/files/**/*".into()),
            ];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/project:task/file.txt"),
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/task/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/project:task/files/**/*"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }

        #[test]
        fn converts_env_variables() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.env.insert("FOO".into(), "foo".into());
            task.env.insert("BAR".into(), "bar".into());

            task.outputs = vec![
                OutputPath::ProjectFile("$FOO/file.txt".into()),
                OutputPath::ProjectGlob("${BAR}/files/**/*".into()),
                OutputPath::WorkspaceFile("cache/$FOO/file.txt".into()),
                OutputPath::WorkspaceGlob("cache/${BAR}/files/**/*".into()),
            ];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_outputs(&task).unwrap(),
                ExpandedResult {
                    files: vec![
                        WorkspaceRelativePathBuf::from("project/source/foo/file.txt"),
                        WorkspaceRelativePathBuf::from("cache/foo/file.txt"),
                    ],
                    globs: vec![
                        WorkspaceRelativePathBuf::from("project/source/bar/files/**/*"),
                        WorkspaceRelativePathBuf::from("cache/bar/files/**/*"),
                    ],
                    ..ExpandedResult::default()
                }
            );
        }
    }

    mod script {
        use super::*;

        #[test]
        fn passes_through() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("bin --foo -az bar".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin --foo -az bar"
            );
        }

        #[test]
        fn replaces_one_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("$project/bin --foo -az bar".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "project/bin --foo -az bar"
            );
        }

        #[test]
        fn replaces_two_vars() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("$project/bin/$task --foo -az bar".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "project/bin/task --foo -az bar"
            );
        }

        #[test]
        fn inherits_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("$FOO".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_script(&mut task).unwrap(), "$FOO");
            assert_eq!(task.input_env, FxHashSet::from_iter(["FOO".into()]));
        }

        #[test]
        fn doesnt_inherit_inputs_from_env_var() {
            let sandbox = create_empty_sandbox();
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("$FOO".into());
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(expander.expand_script(&mut task).unwrap(), "$FOO");
            assert!(task.input_env.is_empty());
        }

        #[test]
        fn inherits_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("bin @group(all)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin ./config.yml ./dir/subdir ./*.md ./**/*.json"
            );
            assert_eq!(
                task.input_files,
                FxHashSet::from_iter([
                    "project/source/config.yml".into(),
                    "project/source/dir/subdir".into()
                ])
            );
            assert_eq!(
                task.input_globs,
                FxHashSet::from_iter([
                    "project/source/**/*.json".into(),
                    "project/source/*.md".into()
                ])
            );
        }

        #[test]
        fn doesnt_inherit_inputs_from_token_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("bin @group(all)".into());
            task.options.infer_inputs = false;

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin ./config.yml ./dir/subdir ./*.md ./**/*.json"
            );
            assert!(task.input_files.is_empty());
            assert!(task.input_globs.is_empty());
        }

        #[test]
        fn supports_out_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("bin --foo -az @out(0)".into());
            task.outputs = vec![OutputPath::ProjectGlob("**/*.json".into())];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin --foo -az ./**/*.json"
            );
        }

        #[test]
        fn supports_in_func() {
            let sandbox = create_sandbox("file-group");
            let project = create_project(sandbox.path());
            let mut task = create_task();

            task.script = Some("bin --foo -az @in(0) @in(1)".into());
            task.inputs = vec![
                InputPath::ProjectFile("docs.md".into()),
                InputPath::ProjectFile("other/file.json".into()),
            ];

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin --foo -az ./docs.md ./other/file.json"
            );
        }

        #[test]
        fn supports_meta_func() {
            let sandbox = create_empty_sandbox();
            let mut project = create_project(sandbox.path());

            project
                .config
                .project
                .get_or_insert(Default::default())
                .name = Some("name".into());

            let mut task = create_task();

            task.script = Some("bin --name @meta(name)".into());

            let context = create_context(sandbox.path());
            let mut expander = TokenExpander::new(&project, &context);

            assert_eq!(
                expander.expand_script(&mut task).unwrap(),
                "bin --name name"
            );
        }
    }
}
