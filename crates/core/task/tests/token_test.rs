use moon_config::{ProjectConfig, ProjectLanguage, ProjectType, TaskConfig};
use moon_task::test::{create_expanded_task, create_file_groups, create_initial_task};
use moon_task::{ResolverData, TokenResolver};
use moon_test_utils::get_fixtures_path;
use moon_utils::{glob, string_vec};
use std::path::PathBuf;

fn get_workspace_root() -> PathBuf {
    get_fixtures_path("base")
}

fn get_project_root() -> PathBuf {
    get_workspace_root().join("files-and-dirs")
}

#[test]
#[should_panic(expected = "UnknownFileGroup(\"@dirs(unknown)\", \"unknown\")")]
fn errors_for_unknown_file_group() {
    let project_root = get_project_root();
    let project_config = ProjectConfig::new(&project_root);
    let workspace_root = get_workspace_root();
    let file_groups = create_file_groups();
    let metadata = ResolverData::new(
        &file_groups,
        &workspace_root,
        &project_root,
        &project_config,
    );
    let resolver = TokenResolver::for_args(&metadata);
    let task = create_initial_task(None);

    resolver
        .resolve(&string_vec!["@dirs(unknown)"], &task)
        .unwrap();
}

#[test]
#[should_panic(expected = "NoGlobs(\"no_globs\")")]
fn errors_if_no_globs_in_file_group() {
    let project_root = get_project_root();
    let project_config = ProjectConfig::new(&project_root);
    let workspace_root = get_workspace_root();
    let file_groups = create_file_groups();
    let metadata = ResolverData::new(
        &file_groups,
        &workspace_root,
        &project_root,
        &project_config,
    );
    let resolver = TokenResolver::for_args(&metadata);
    let task = create_initial_task(None);

    resolver
        .resolve(&string_vec!["@globs(no_globs)"], &task)
        .unwrap();
}

#[test]
fn doesnt_match_when_not_alone() {
    let project_root = get_project_root();
    let project_config = ProjectConfig::new(&project_root);
    let workspace_root = get_workspace_root();
    let file_groups = create_file_groups();
    let metadata = ResolverData::new(
        &file_groups,
        &workspace_root,
        &project_root,
        &project_config,
    );
    let resolver = TokenResolver::for_args(&metadata);
    let task = create_initial_task(None);

    assert_eq!(
        resolver
            .resolve(&string_vec!["foo/@dirs(static)/bar"], &task)
            .unwrap(),
        (vec![project_root.join("foo/@dirs(static)/bar")], vec![])
    );
}

mod in_token {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidIndexType(\"@in(abc)\", \"abc\")")]
    fn errors_for_invalid_index_format() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        resolver.resolve(&string_vec!["@in(abc)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidInIndex(\"@in(5)\", 5)")]
    fn errors_for_index_out_of_bounds() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        resolver.resolve(&string_vec!["@in(5)"], &task).unwrap();
    }
}

mod out_token {
    use super::*;
    #[test]
    #[should_panic(expected = "InvalidIndexType(\"@out(abc)\", \"abc\")")]
    fn errors_for_invalid_index_format() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                outputs: Some(string_vec!["dir", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        resolver.resolve(&string_vec!["@out(abc)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidOutIndex(\"@out(5)\", 5)")]
    fn errors_for_index_out_of_bounds() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                outputs: Some(string_vec!["dir", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        resolver.resolve(&string_vec!["@out(5)"], &task).unwrap();
    }
}

mod args {
    use super::*;
    use moon_config::PlatformType;

    #[test]
    fn supports_dirs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(static)"], &task)
                .unwrap(),
            (
                vec![project_root.join("dir"), project_root.join("dir/subdir")],
                vec![]
            )
        );
    }

    #[test]
    fn supports_dirs_with_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(dirs_glob)"], &task)
                .unwrap(),
            (
                vec![project_root.join("dir"), project_root.join("dir/subdir")],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files_with_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(files_glob)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@globs(globs)"], &task)
                .unwrap(),
            (
                vec![],
                vec![
                    glob::normalize(project_root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project_root.join("*.js")).unwrap()
                ]
            )
        );
    }

    #[test]
    fn supports_in_paths() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["dir/**/*", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            resolver.resolve(&string_vec!["@in(1)"], &task).unwrap(),
            (vec![project_root.join("file.ts")], vec![])
        );
    }

    #[test]
    fn supports_in_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                inputs: Some(string_vec!["src/**/*", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            resolver.resolve(&string_vec!["@in(0)"], &task).unwrap(),
            (
                vec![],
                vec![glob::normalize(project_root.join("src/**/*")).unwrap()]
            )
        );
    }

    #[test]
    fn supports_out_paths() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let task = create_expanded_task(
            &workspace_root,
            &project_root,
            Some(TaskConfig {
                outputs: Some(string_vec!["dir/", "file.ts"]),
                ..TaskConfig::default()
            }),
        )
        .unwrap();

        assert_eq!(
            resolver
                .resolve(&string_vec!["@out(0)", "@out(1)"], &task)
                .unwrap(),
            (
                vec![project_root.join("dir"), project_root.join("file.ts")],
                vec![]
            )
        );
    }

    #[test]
    fn supports_root() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@root(static)"], &task)
                .unwrap(),
            (vec![project_root.join("dir")], vec![])
        );
    }

    #[test]
    fn supports_vars() {
        let project_root = get_project_root();
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let project_config = ProjectConfig {
            language: ProjectLanguage::JavaScript,
            type_of: ProjectType::Tool,
            ..ProjectConfig::default()
        };
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_args(&metadata);

        let mut task = create_expanded_task(&workspace_root, &project_root, None).unwrap();
        task.platform = PlatformType::Node;

        assert_eq!(
            resolver.resolve_var("$language", &task).unwrap(),
            "javascript"
        );

        assert_eq!(resolver.resolve_var("$project", &task).unwrap(), "project");

        assert_eq!(
            resolver.resolve_var("$projectRoot", &task).unwrap(),
            project_root.to_string_lossy()
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

mod inputs {
    use super::*;

    #[test]
    fn supports_dirs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(static)"], &task)
                .unwrap(),
            (
                vec![project_root.join("dir"), project_root.join("dir/subdir")],
                vec![]
            )
        );
    }

    #[test]
    fn supports_dirs_with_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@dirs(dirs_glob)"], &task)
                .unwrap(),
            (
                vec![project_root.join("dir"), project_root.join("dir/subdir")],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_files_with_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        let mut files = resolver
            .resolve(&string_vec!["@files(files_glob)"], &task)
            .unwrap();
        files.0.sort();

        assert_eq!(
            files,
            (
                vec![
                    project_root.join("dir/other.tsx"),
                    project_root.join("dir/subdir/another.ts"),
                    project_root.join("file.ts"),
                ],
                vec![]
            )
        );
    }

    #[test]
    fn supports_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@globs(globs)"], &task)
                .unwrap(),
            (
                vec![],
                vec![
                    glob::normalize(project_root.join("**/*.{ts,tsx}")).unwrap(),
                    glob::normalize(project_root.join("*.js")).unwrap()
                ]
            ),
        );
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@in\", \"inputs\")")]
    fn doesnt_support_in() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        resolver.resolve(&string_vec!["@in(0)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@out\", \"inputs\")")]
    fn doesnt_support_out() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        resolver.resolve(&string_vec!["@out(0)"], &task).unwrap();
    }

    #[test]
    fn supports_root() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_inputs(&metadata);
        let task = create_initial_task(None);

        assert_eq!(
            resolver
                .resolve(&string_vec!["@root(static)"], &task)
                .unwrap(),
            (vec![project_root.join("dir")], vec![]),
        );
    }
}

mod outputs {
    use super::*;

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@dirs\", \"outputs\")")]
    fn doesnt_support_dirs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver
            .resolve(&string_vec!["@dirs(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@files\", \"outputs\")")]
    fn doesnt_support_files() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver
            .resolve(&string_vec!["@files(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@globs\", \"outputs\")")]
    fn doesnt_support_globs() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver
            .resolve(&string_vec!["@globs(globs)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@group\", \"outputs\")")]
    fn doesnt_support_group() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver
            .resolve(&string_vec!["@group(group)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@in\", \"outputs\")")]
    fn doesnt_support_in() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver.resolve(&string_vec!["@in(0)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@out\", \"outputs\")")]
    fn doesnt_support_out() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver.resolve(&string_vec!["@out(0)"], &task).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"@root\", \"outputs\")")]
    fn doesnt_support_root() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver
            .resolve(&string_vec!["@root(static)"], &task)
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidTokenContext(\"$var\", \"outputs\")")]
    fn doesnt_support_vars() {
        let project_root = get_project_root();
        let project_config = ProjectConfig::new(&project_root);
        let workspace_root = get_workspace_root();
        let file_groups = create_file_groups();
        let metadata = ResolverData::new(
            &file_groups,
            &workspace_root,
            &project_root,
            &project_config,
        );
        let resolver = TokenResolver::for_outputs(&metadata);
        let task = create_initial_task(None);

        resolver.resolve(&string_vec!["$project"], &task).unwrap();
    }
}
