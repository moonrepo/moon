mod action;
mod context;

use moon_logger::color;
use moon_utils::process;
use std::path::Path;

pub use action::*;
pub use context::*;

pub fn format_running_command(
    command: &str,
    args: &[String],
    working_dir: &Path,
    workspace_root: &Path,
) -> String {
    let command_line = if args.is_empty() {
        command.to_owned()
    } else {
        format!("{} {}", command, process::join_args(args))
    };

    let target_dir = if working_dir == workspace_root {
        String::from("workspace")
    } else {
        format!(
            ".{}{}",
            std::path::MAIN_SEPARATOR,
            working_dir
                .strip_prefix(&workspace_root)
                .unwrap()
                .to_string_lossy(),
        )
    };

    let suffix = format!("(in {})", target_dir);
    let message = format!("{} {}", command_line, color::muted(suffix));

    color::muted_light(message)
}
