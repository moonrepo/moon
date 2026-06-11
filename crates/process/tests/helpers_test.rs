use moon_process::format_command_line;
use std::path::Path;

mod format_command_line {
    use super::*;

    #[test]
    fn labels_workspace_root() {
        let line = format_command_line("git status", Path::new("/root"), Path::new("/root"));

        assert!(line.contains("git status"));
        assert!(line.contains("(in workspace)"));
    }

    #[test]
    fn labels_relative_working_dir() {
        // Join with the native separator, as a forward slash literal
        // would be preserved as-is on Windows and never match
        let workspace_root = Path::new("/root");
        let working_dir = workspace_root.join("packages").join("foo");

        let line = format_command_line("git status", workspace_root, &working_dir);

        assert!(line.contains(&format!(
            "(in .{}packages{}foo)",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        )));
    }

    #[test]
    fn labels_unrelated_working_dir() {
        let line = format_command_line("git status", Path::new("/root"), Path::new("/elsewhere"));

        assert!(line.contains("(in .)"));
    }

    #[test]
    fn trims_command() {
        let line = format_command_line("  git status  ", Path::new("/root"), Path::new("/root"));

        assert!(line.contains("git status"));
        assert!(!line.contains("  git status"));
    }
}

#[cfg(unix)]
mod find_command {
    use moon_process::find_command_on_path;

    #[test]
    fn finds_common_binaries() {
        assert!(find_command_on_path("sh").is_some());
        assert!(find_command_on_path("nonexistent_binary_zzz").is_none());
    }
}
