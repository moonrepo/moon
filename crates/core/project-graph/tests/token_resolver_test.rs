use moon_config::{InheritedTasksManager, PlatformType, ProjectLanguage, ProjectType, TaskConfig};
use moon_project::Project;
use moon_project_graph::{TokenContext, TokenResolver};
use moon_target::Target;
use moon_task::{FileGroup, Task};
use moon_test_utils::get_fixtures_path;
use moon_utils::{glob, string_vec};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

pub fn create_file_groups() -> FxHashMap<String, FileGroup> {
    let mut map = FxHashMap::default();

    map.insert(
        "static".into(),
        FileGroup::new(
            "static",
            string_vec![
                "file.ts",
                "dir",
                "dir/other.tsx",
                "dir/subdir",
                "dir/subdir/another.ts",
            ],
        ),
    );

    map.insert(
        "dirs_glob".into(),
        FileGroup::new("dirs_glob", string_vec!["**/*"]),
    );

    map.insert(
        "files_glob".into(),
        FileGroup::new("files_glob", string_vec!["**/*.{ts,tsx}"]),
    );

    map.insert(
        "globs".into(),
        FileGroup::new("globs", string_vec!["**/*.{ts,tsx}", "*.js"]),
    );

    map.insert(
        "no_globs".into(),
        FileGroup::new("no_globs", string_vec!["config.js"]),
    );

    map
}

fn get_workspace_root() -> PathBuf {
    get_fixtures_path("base")
}

fn create_project(workspace_root: &Path) -> Project {
    let mut project = Project::new(
        "project",
        "files-and-dirs",
        workspace_root,
        &InheritedTasksManager::default(),
        |_| ProjectLanguage::Unknown,
    )
    .unwrap();
    project.file_groups = create_file_groups();
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
    for input in &task.inputs {
        if glob::is_glob(input) {
            task.input_globs
                .insert(glob::normalize(project.root.join(input)).unwrap());
        } else {
            task.input_paths.insert(project.root.join(input));
        }
    }

    for output in &task.outputs {
        if glob::is_glob(output) {
            task.output_globs
                .insert(glob::normalize(project.root.join(output)).unwrap());
        } else {
            task.output_paths.insert(project.root.join(output));
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
        (vec![project.root.join("foo/@dirs(static)/bar")], vec![])
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
            inputs: Some(string_vec!["dir/**/*", "file.ts"]),
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
            inputs: Some(string_vec!["dir/**/*", "file.ts"]),
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
            outputs: Some(string_vec!["dir", "file.ts"]),
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
            outputs: Some(string_vec!["dir", "file.ts"]),
            ..TaskConfig::default()
        }));

        resolver.resolve(&string_vec!["@out(5)"], &task).unwrap();
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
                vec![project.root.join("dir"), project.root.join("dir/subdir")],
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
                vec![project.root.join("dir"), project.root.join("dir/subdir")],
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
                    project.root.join("dir/other.tsx"),
                    project.root.join("dir/subdir/another.ts"),
                    project.root.join("file.ts"),
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
                    project.root.join("dir/other.tsx"),
                    project.root.join("dir/subdir/another.ts"),
                    project.root.join("file.ts"),
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
                    glob::normalize(project.root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project.root.join("*.js")).unwrap()
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
            inputs: Some(string_vec!["dir/**/*", "file.ts"]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver.resolve(&string_vec!["@in(1)"], &task).unwrap(),
            (vec![project.root.join("file.ts")], vec![])
        );
    }

    #[test]
    fn supports_in_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let mut task = create_task(Some(TaskConfig {
            inputs: Some(string_vec!["src/**/*", "file.ts"]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver.resolve(&string_vec!["@in(0)"], &task).unwrap(),
            (
                vec![],
                vec![glob::normalize(project.root.join("src/**/*")).unwrap()]
            )
        );
    }

    #[test]
    fn supports_out_paths() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);
        let mut task = create_task(Some(TaskConfig {
            outputs: Some(string_vec!["dir/", "file.ts"]),
            ..TaskConfig::default()
        }));

        expand_task(&project, &mut task);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@out(0)", "@out(1)"], &task)
                .unwrap(),
            (
                vec![project.root.join("dir"), project.root.join("file.ts")],
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
            (vec![project.root.join("dir")], vec![])
        );
    }

    #[test]
    fn supports_vars() {
        let workspace_root = get_workspace_root();
        let mut project = create_project(&workspace_root);
        project.language = ProjectLanguage::JavaScript;
        project.type_of = ProjectType::Tool;

        let resolver = TokenResolver::new(TokenContext::Args, &project, &workspace_root);

        let mut task = create_task(None);
        task.platform = PlatformType::Node;

        assert_eq!(
            resolver.resolve_var("$language", &task).unwrap(),
            "javascript"
        );

        assert_eq!(resolver.resolve_var("$project", &task).unwrap(), "project");

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
                vec![project.root.join("dir"), project.root.join("dir/subdir")],
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
                vec![project.root.join("dir"), project.root.join("dir/subdir")],
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
                    project.root.join("dir/other.tsx"),
                    project.root.join("dir/subdir/another.ts"),
                    project.root.join("file.ts"),
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
                    project.root.join("dir/other.tsx"),
                    project.root.join("dir/subdir/another.ts"),
                    project.root.join("file.ts"),
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
                    glob::normalize(project.root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project.root.join("*.js")).unwrap()
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
            (vec![project.root.join("dir")], vec![]),
        );
    }

    #[test]
    fn converts_naked_dir_to_glob() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Inputs, &project, &workspace_root);
        let task = create_task(None);

        assert_eq!(
            resolver.resolve(&string_vec!["dir"], &task).unwrap(),
            (
                vec![],
                vec![if cfg!(windows) {
                    glob::normalize(project.root.join("dir/**/*")).unwrap()
                } else {
                    project.root.join("dir/**/*").to_string_lossy().to_string()
                }]
            ),
        );
    }
}

mod resolve_outputs {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@dirs\", \"outputs\")")]
    fn doesnt_support_dirs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver
            .resolve(&string_vec!["@dirs(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@files\", \"outputs\")")]
    fn doesnt_support_files() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@globs\", \"outputs\")")]
    fn doesnt_support_globs() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver
            .resolve(&string_vec!["@globs(globs)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@group\", \"outputs\")")]
    fn doesnt_support_group() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver
            .resolve(&string_vec!["@group(group)"], &task)
            .unwrap();
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
    #[should_panic(expected = "InvalidTokenContext(\"@root\", \"outputs\")")]
    fn doesnt_support_root() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver
            .resolve(&string_vec!["@root(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"$var\", \"outputs\")")]
    fn doesnt_support_vars() {
        let workspace_root = get_workspace_root();
        let project = create_project(&workspace_root);
        let resolver = TokenResolver::new(TokenContext::Outputs, &project, &workspace_root);
        let task = create_task(None);

        resolver.resolve(&string_vec!["$project"], &task).unwrap();
    }
}
