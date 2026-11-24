use moon_common::path::{
    RelativeFrom, WorkspaceRelativeGlob, WorkspaceRelativePathBuf, escape_glob,
    expand_to_workspace_relative_glob,
};

fn escaped(path: &str, escaped: &str) -> WorkspaceRelativeGlob {
    WorkspaceRelativeGlob {
        path: path.into(),
        escaped: Some(escaped.into()),
    }
}

mod escape_glob {
    use super::*;

    #[test]
    fn returns_paths_without_meta_chars_as_is() {
        assert_eq!(escape_glob("packages/lib-a/src"), "packages/lib-a/src");
    }

    #[test]
    fn escapes_meta_chars() {
        assert_eq!(escape_glob("packages/x:y"), "packages/x\\:y");
        assert_eq!(escape_glob("packages/[lib]"), "packages/\\[lib\\]");
        assert_eq!(escape_glob("apps/app (old)"), "apps/app \\(old\\)");
        assert_eq!(escape_glob("a{b,c}"), "a\\{b\\,c\\}");
    }
}

mod workspace_relative_glob {
    use super::*;

    #[test]
    fn isnt_escaped_without_meta_chars() {
        let glob = WorkspaceRelativeGlob::join("packages/lib", "src/**/*");

        assert_eq!(glob, WorkspaceRelativeGlob::from("packages/lib/src/**/*"));
        assert_eq!(glob.as_glob(), "packages/lib/src/**/*");
    }

    #[test]
    fn escapes_the_base_only() {
        let glob = WorkspaceRelativeGlob::join("packages/[lib]", "[a-z]/*");

        assert_eq!(glob.path, "packages/[lib]/[a-z]/*");
        assert_eq!(glob.as_glob(), "packages/\\[lib\\]/[a-z]/*");
    }

    #[test]
    fn keeps_negation_at_the_start() {
        assert_eq!(
            WorkspaceRelativeGlob::join("packages/[lib]", "!dist/**"),
            escaped("!packages/[lib]/dist/**", "!packages/\\[lib\\]/dist/**")
        );
    }

    #[test]
    fn maps_both_forms() {
        let glob = WorkspaceRelativeGlob::join("packages/[lib]", "$name/*")
            .map(|path| Ok::<_, ()>(path.as_str().replace("$name", "src").into()))
            .unwrap();

        assert_eq!(
            glob,
            escaped("packages/[lib]/src/*", "packages/\\[lib\\]/src/*")
        );
    }
}

// https://github.com/moonrepo/moon/issues/2818 (rqk8s7)
mod expand_to_workspace_relative_glob {
    use super::*;

    #[test]
    fn escapes_project_source_only() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/[lib]"), "src/**/*"),
            escaped("packages/[lib]/src/**/*", "packages/\\[lib\\]/src/**/*")
        );
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/x:y"), "*.{ts,tsx}"),
            escaped("packages/x:y/*.{ts,tsx}", "packages/x\\:y/*.{ts,tsx}")
        );
    }

    #[test]
    fn escapes_negated_globs() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/[lib]"), "!dist/**"),
            escaped("!packages/[lib]/dist/**", "!packages/\\[lib\\]/dist/**")
        );
    }

    #[test]
    fn standardizes_separators_before_escaping() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages\\[lib]"), "src\\*"),
            escaped("packages/[lib]/src/*", "packages/\\[lib\\]/src/*")
        );
    }

    #[test]
    fn doesnt_prefix_root_level_or_workspace_globs() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("."), "src/**/*"),
            WorkspaceRelativeGlob::from(WorkspaceRelativePathBuf::from("src/**/*"))
        );
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Workspace, "[a-z]/**/*"),
            WorkspaceRelativeGlob::from("[a-z]/**/*")
        );
    }
}
