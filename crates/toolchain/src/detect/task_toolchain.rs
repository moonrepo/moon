use moon_common::Id;
use regex::Regex;
use std::sync::OnceLock;

pub static BUN_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static DENO_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static PYTHON_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static RUST_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static NODE_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static UNIX_SYSTEM_COMMANDS: OnceLock<Regex> = OnceLock::new();
pub static WINDOWS_SYSTEM_COMMANDS: OnceLock<Regex> = OnceLock::new();

pub fn is_system_command(command: &str) -> bool {
    let unix = UNIX_SYSTEM_COMMANDS.get_or_init(|| {
        Regex::new(
            "^(bash|cat|cd|chmod|cp|docker|echo|find|git|grep|make|mkdir|mv|pwd|rm|rsync|svn)$",
        )
        .unwrap()
    });

    let windows = WINDOWS_SYSTEM_COMMANDS.get_or_init(|| Regex::new(
        "^(cd|cmd|cmd.exe|copy|del|dir|echo|erase|find|git|mkdir|move|rd|rename|replace|rmdir|svn|xcopy|pwsh|pwsh.exe|powershell|powershell.exe)$",
    )
    .unwrap());

    unix.is_match(command) || windows.is_match(command)
}

pub fn detect_task_toolchains(command: &str, enabled_toolchains: &[Id]) -> Vec<Id> {
    let mut toolchains = vec![];
    let detectors = vec![
        (
            Id::raw("bun"),
            BUN_COMMANDS.get_or_init(|| Regex::new("^(bun|bunx)$").unwrap()),
        ),
        (
            Id::raw("deno"),
            DENO_COMMANDS.get_or_init(|| Regex::new("^(deno)$").unwrap()),
        ),
        (
            Id::raw("python"),
            PYTHON_COMMANDS
                .get_or_init(|| Regex::new("^(python|python3|python-3|pip|pip3|pip-3)$").unwrap()),
        ),
        (
            Id::raw("rust"),
            RUST_COMMANDS
                .get_or_init(|| Regex::new("^(rust-|rustc|rustdoc|rustfmt|rustup|cargo)").unwrap()),
        ),
        (
            Id::raw("node"),
            NODE_COMMANDS.get_or_init(|| {
                Regex::new("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap()
            }),
        ),
    ];

    // Detect the toolchain based on the task command
    for (id, pattern) in detectors {
        if pattern.is_match(command) && enabled_toolchains.contains(&id) {
            toolchains.push(id);
            break;
        }
    }

    // If no toolchain detected or inherited, fallback to the system
    if is_system_command(command) {
        toolchains.push(Id::raw("system"));
    }

    toolchains
}
