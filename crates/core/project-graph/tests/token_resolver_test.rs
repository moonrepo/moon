use moon_common::Id;
use moon_config::{
    InheritedTasksManager, InputPath, LanguageType, OutputPath, PlatformType, ProjectType,
    TaskConfig,
};
use moon_file_group::FileGroup;
use moon_project::Project;
use moon_project_graph::{TokenContext, TokenResolver};
use moon_target::Target;
use moon_task::Task;
use moon_test_utils::{
    create_workspace_paths_with_prefix, get_fixtures_path, predicates::prelude::*,
};
use rustc_hash::FxHashMap;
use starbase_utils::{glob, string_vec};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn create_file_groups(source: &str) -> FxHashMap<Id, FileGroup> {
    let mut map = FxHashMap::default();

    map.insert(
        "static".into(),
        FileGroup::new_with_source(
            "static",
            create_workspace_paths_with_prefix(
                source,
                [
                    "file.ts",
                    "dir",
                    "dir/other.tsx",
                    "dir/subdir",
                    "dir/subdir/another.ts",
                ],
            ),
        )
        .unwrap(),
    );

    map.insert(
        "dirs_glob".into(),
        FileGroup::new_with_source(
            "dirs_glob",
            create_workspace_paths_with_prefix(source, ["**/*"]),
        )
        .unwrap(),
    );

    map.insert(
        "files_glob".into(),
        FileGroup::new_with_source(
            "files_glob",
            create_workspace_paths_with_prefix(source, ["**/*.{ts,tsx}"]),
        )
        .unwrap(),
    );

    map.insert(
        "globs".into(),
        FileGroup::new_with_source(
            "globs",
            create_workspace_paths_with_prefix(source, ["**/*.{ts,tsx}", "*.js"]),
        )
        .unwrap(),
    );

    map.insert(
        "no_globs".into(),
        FileGroup::new_with_source(
            "no_globs",
            create_workspace_paths_with_prefix(source, ["config.js"]),
        )
        .unwrap(),
    );

    map
}

fn get_workspace_root() -> PathBuf {
    get_fixtures_path("base")
}

fn create_project(workspace_root: &Path) -> Project {
    let mut project = Project::new(
        &Id::raw("project"),
        "files-and-dirs",
        workspace_root,
        &InheritedTasksManager::default(),
        |_| LanguageType::Unknown,
    )
    .unwrap();
    project.file_groups = create_file_groups("files-and-dirs");
    project
}

pub fn create_task(config: Option<TaskConfig>) -> Task {
    Task::from_config(
        Target::new("project", "task").unwrap(),
        &config.unwrap_or_default(),
    )
    .unwrap()
}

pub fn expand_task(project: &Project, task: &mut Task) {
    let project_source = PathBuf::from(&project.source);

    for input in &task.inputs {
        if glob::is_glob(input) {
            task.input_globs
                .insert(glob::normalize(project_source.join(input.as_str())).unwrap());
        } else {
            task.input_paths.insert(project_source.join(input.as_str()));
        }
    }

    for output in &task.outputs {
        if glob::is_glob(output) {
            task.output_globs
                .insert(glob::normalize(project_source.join(output.as_str())).unwrap());
        } else {
            task.output_paths
                .insert(project_source.join(output.as_str()));
        }
    }
}

#[test]
#[should_panic(expected = "UnknownFileGroup(\"@dirs(unknown)\", \"unknown\")")]
fn errors_for_unknown_file_group() {
    let workspace_root = get_workspace_root();
    let project = create_project(&workspace_root);
    let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
    let task = create_task(None);

    resolver
        .resolve(&string_vec!["@dirs(unknown)"], &task)
        .unwrap();
}

#[test]
#[should_panic(expected = "NoGlobs(\"no_globs\")")]
fn errors_if_no_globs_in_file_group() {
    let workspace_root = get_workspace_root();
    let project = create_project(&workspace_root);
    let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
    let task = create_task(None);

    resolver
        .resolve(&string_vec!["@globs(no_globs)"], &task)
        .unwrap();
}

#[test]
fn doesnt_match_when_not_alone() {
    let workspace_root = get_workspace_root();
    let project = create_project(&workspace_root);
    let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
    let task = create_task(None);

    assert_eq!(
        resolver
            .resolve(&string_vec!["foo/@dirs(static)/bar"], &task)
            .unwrap(),
        (
            vec![PathBuf::from(project.source).join("foo/@dirs(static)/bar")],
            vec![]
        )
    );
}

mod in_token {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidIndexType(\"@in(abc)\", \"abc\")")]
    fn errors_for_invalid_index_format() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(Some(TaskConfig {
            inputs: Some(vec![
                InputPath::from_str("dir/**/*").unwrap(),
                InputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@in(abc)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidInIndex(\"@in(5)\", 5)")]
    fn errors_for_index_out_of_bounds() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(Some(TaskConfig {
            inputs: Some(vec![
                InputPath::from_str("dir/**/*").unwrap(),
                InputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@in(5)"], &task).unwrap();
    }
}

mod out_token {
    use super::*;
    #[test]
    #[should_panic(expected = "InvalidIndexType(\"@out(abc)\", \"abc\")")]
    fn errors_for_invalid_index_format() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(Some(TaskConfig {
            outputs: Some(vec![
                OutputPath::from_str("dir").unwrap(),
                OutputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@out(abc)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidOutIndex(\"@out(5)\", 5)")]
    fn errors_for_index_out_of_bounds() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(Some(TaskConfig {
            outputs: Some(vec![
                OutputPath::from_str("dir").unwrap(),
                OutputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@out(5)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidOutNoTokenFunctions(\"@out(0)\")")]
    fn errors_for_referencing_token_func() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(Some(TaskConfig {
            outputs: Some(vec![OutputPath::from_str("@group(name)").unwrap()]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@out(0)"], &task).unwrap();
    }
}

mod resolve_command {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@in\", \"command\")")]
    fn doesnt_support_functions() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);

        let resolver = TokenResolver::new(TokenContext::Command, &project, &workspace_root);
        let mut task = create_task(None);
        task.command = "@in(0)".into();

        resolver.resolve_command(&task).unwrap();
    }

    #[test]
    fn supports_vars() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);

        let resolver = TokenResolver::new(TokenContext::Command, &project, &workspace_root);
        let mut task = create_task(None);
        task.command = "$language/script.sh".into();

        assert_eq!(
            resolver.resolve_command(&task).unwrap(),
            "unknown/script.sh"
        );
    }
}

mod resolve_args {
    use super::*;

    #[test]
    fn supports_dirs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(static)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_dirs_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(dirs_glob)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(files_glob)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@globs(globs)"], &task)
                .unwrap(),
            (
                vec![],
                vec![
                    glob::normalize(PathBuf::from(&project.source).join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(PathBuf::from(&project.source).join("*.js")).unwrap()
                ]
            )
        );
    }

    #[test]
    fn supports_in_paths() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let mut task = create_task(Some(TaskConfig {
            inputs: Some(vec![
                InputPath::from_str("dir/**/*").unwrap(),
                InputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver.resolve(&string_vec!["@in(1)"], &task).unwrap(),
            (vec![PathBuf::from(&project.source).join("file.ts")], vec![])
        );
    }

    #[test]
    fn supports_in_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let mut task = create_task(Some(TaskConfig {
            inputs: Some(vec![
                InputPath::from_str("src/**/*").unwrap(),
                InputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver.resolve(&string_vec!["@in(0)"], &task).unwrap(),
            (
                vec![],
                vec![glob::normalize(PathBuf::from(&project.source).join("src/**/*")).unwrap()]
            )
        );
    }

    #[test]
    fn supports_out_paths() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let mut task = create_task(Some(TaskConfig {
            outputs: Some(vec![
                OutputPath::from_str("dir/").unwrap(),
                OutputPath::from_str("file.ts").unwrap(),
            ]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@out(0)", "@out(1)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("file.ts")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_root() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@root(static)"], &task)
                .unwrap(),
            (vec![PathBuf::from(&project.source).join("dir")], vec![])
        );
    }

    #[test]
    fn supports_vars() {
        let workspace_root = get_workspace_root();
        let mut project = create_project(&workspace_root);
        project.language = LanguageType::JavaScript;
        project.type_of = ProjectType::Tool;
        project.alias = Some("some-alias".into());

        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);

        let mut task = create_task(None);
        task.platform = PlatformType::Node;

        assert_eq!(
            resolver.resolve_var("$language", &task).unwrap(),
            "javascript"
        );

        assert_eq!(resolver.resolve_var("$project", &task).unwrap(), "project");

        assert_eq!(
            resolver.resolve_var("$projectAlias", &task).unwrap(),
            "some-alias"
        );

        assert_eq!(
            resolver.resolve_var("$projectRoot", &task).unwrap(),
            project.root.to_string_lossy()
        );

        assert_eq!(
            resolver.resolve_var("$projectSource", &task).unwrap(),
            "files-and-dirs"
        );

        assert_eq!(resolver.resolve_var("$projectType", &task).unwrap(), "tool");

        assert_eq!(
            resolver.resolve_var("$target", &task).unwrap(),
            "project:task"
        );

        assert_eq!(resolver.resolve_var("$task", &task).unwrap(), "task");

        assert_eq!(
            resolver.resolve_var("$taskPlatform", &task).unwrap(),
            "node"
        );

        assert_eq!(resolver.resolve_var("$taskType", &task).unwrap(), "test");

        assert_eq!(
            resolver.resolve_var("$workspaceRoot", &task).unwrap(),
            workspace_root.to_string_lossy()
        );

        assert!(predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}")
            .unwrap()
            .eval(&resolver.resolve_var("$date", &task).unwrap()));

        assert!(predicate::str::is_match("[0-9]{2}:[0-9]{2}:[0-9]{2}")
            .unwrap()
            .eval(&resolver.resolve_var("$time", &task).unwrap()));

        assert!(
            predicate::str::is_match("[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}:[0-9]{2}:[0-9]{2}")
                .unwrap()
                .eval(&resolver.resolve_var("$datetime", &task).unwrap())
        );

        assert!(predicate::str::is_match("[0-9]{10}")
            .unwrap()
            .eval(&resolver.resolve_var("$timestamp", &task).unwrap()));

        // Multiple vars
        assert_eq!(
            resolver
                .resolve_vars("$language-$taskPlatform-project", &task)
                .unwrap(),
            "javascript-node-project"
        );

        // Unknown var
        assert_eq!(resolver.resolve_var("$unknown", &task).unwrap(), "$unknown");
    }
}

mod resolve_inputs {
    use super::*;

    #[test]
    fn supports_dirs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(static)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_dirs_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(dirs_glob)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(files_glob)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@globs(globs)"], &task)
                .unwrap(),
            (
                vec![],
                vec![
                    glob::normalize(PathBuf::from(&project.source).join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(PathBuf::from(&project.source).join("*.js")).unwrap()
                ]
            ),
        );
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@in\", \"inputs\")")]
    fn doesnt_support_in() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["@in(0)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@out\", \"inputs\")")]
    fn doesnt_support_out() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["@out(0)"], &task).unwrap();
    }

    #[test]
    fn supports_root() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@root(static)"], &task)
                .unwrap(),
            (vec![PathBuf::from(&project.source).join("dir")], vec![]),
        );
    }

    #[test]
    fn converts_naked_dir_to_glob() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve_inputs(&[&InputPath::ProjectFile("dir".into())], &task)
                .unwrap(),
            (
                vec![],
                vec![glob::normalize(PathBuf::from(&project.source).join("dir/**/*")).unwrap()]
            ),
        );
    }
}

mod resolve_outputs {
    use super::*;

    #[test]
    fn supports_dirs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(static)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_dirs_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(dirs_glob)"], &task)
                .unwrap(),
            (
                vec![
                    PathBuf::from(&project.source).join("dir"),
                    PathBuf::from(&project.source).join("dir/subdir")
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files_with_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(files_glob)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    PathBuf::from(&project.source).join("dir/other.tsx"),
                    PathBuf::from(&project.source).join("dir/subdir/another.ts"),
                    PathBuf::from(&project.source).join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@globs(globs)"], &task)
                .unwrap(),
            (
                vec![],
                vec![
                    glob::normalize(PathBuf::from(&project.source).join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(PathBuf::from(&project.source).join("*.js")).unwrap()
                ]
            ),
        );
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@in\", \"outputs\")")]
    fn doesnt_support_in() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["@in(0)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@out\", \"outputs\")")]
    fn doesnt_support_out() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["@out(0)"], &task).unwrap();
    }

    #[test]
    fn supports_root() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@root(static)"], &task)
                .unwrap(),
            (vec![PathBuf::from(&project.source).join("dir")], vec![]),
        );
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"$project\", \"outputs\")")]
    fn doesnt_support_vars() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["$project"], &task).unwrap();
    }
}
