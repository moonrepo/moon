use moon_common::path::{
    RelativeFrom, WorkspaceRelativePathBuf, escape_glob, expand_to_workspace_relative_glob,
};

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

// https://github.com/moonrepo/moon/issues/2818 (rqk8s7)
mod expand_to_workspace_relative_glob {
    use super::*;

    #[test]
    fn escapes_project_source_only() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/[lib]"), "src/**/*"),
            WorkspaceRelativePathBuf::from("packages/\\[lib\\]/src/**/*")
        );
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/x:y"), "*.{ts,tsx}"),
            WorkspaceRelativePathBuf::from("packages/x\\:y/*.{ts,tsx}")
        );
    }

    #[test]
    fn escapes_negated_globs() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages/[lib]"), "!dist/**"),
            WorkspaceRelativePathBuf::from("!packages/\\[lib\\]/dist/**")
        );
    }

    #[test]
    fn standardizes_separators_before_escaping() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("packages\\[lib]"), "src\\*"),
            WorkspaceRelativePathBuf::from("packages/\\[lib\\]/src/*")
        );
    }

    #[test]
    fn doesnt_prefix_root_level_or_workspace_globs() {
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Project("."), "src/**/*"),
            WorkspaceRelativePathBuf::from("src/**/*")
        );
        assert_eq!(
            expand_to_workspace_relative_glob(RelativeFrom::Workspace, "[a-z]/**/*"),
            WorkspaceRelativePathBuf::from("[a-z]/**/*")
        );
    }
}
