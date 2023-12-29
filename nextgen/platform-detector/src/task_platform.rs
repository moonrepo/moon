use moon_config::PlatformType;
use once_cell::sync::Lazy;
use regex::Regex;

static BUN_COMMANDS: Lazy<Regex> = Lazy::new(|| Regex::new("^(bun|bunx)$").unwrap());

static DENO_COMMANDS: Lazy<Regex> = Lazy::new(|| Regex::new("^(deno)$").unwrap());

static RUST_COMMANDS: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(rust-|rustc|rustdoc|rustfmt|rustup|cargo)").unwrap());

static NODE_COMMANDS: Lazy<Regex> =
    Lazy::new(|| Regex::new("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap());

static UNIX_SYSTEM_COMMANDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new("^(bash|cat|cd|chmod|cp|docker|echo|find|git|grep|make|mkdir|mv|pwd|rm|rsync|svn)$")
        .unwrap()
});

static WINDOWS_SYSTEM_COMMANDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "^(cd|cmd|cmd.exe|copy|del|dir|echo|erase|find|git|mkdir|move|rd|rename|replace|rmdir|svn|xcopy|pwsh|pwsh.exe)$",
    )
    .unwrap()
});

fn use_platform_if_enabled(
    platform: PlatformType,
    enabled_platforms: &[PlatformType],
) -> PlatformType {
    match platform {
        PlatformType::Bun if enabled_platforms.contains(&PlatformType::Bun) => return platform,
        PlatformType::Deno if enabled_platforms.contains(&PlatformType::Deno) => return platform,
        PlatformType::Node if enabled_platforms.contains(&PlatformType::Node) => return platform,
        PlatformType::Rust if enabled_platforms.contains(&PlatformType::Rust) => return platform,
        _ => {}
    };

    PlatformType::System
}

pub fn detect_task_platform(command: &str, enabled_platforms: &[PlatformType]) -> PlatformType {
    if BUN_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Bun, enabled_platforms);
    }

    if DENO_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Deno, enabled_platforms);
    }

    if NODE_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Node, enabled_platforms);
    }

    if RUST_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Rust, enabled_platforms);
    }

    if UNIX_SYSTEM_COMMANDS.is_match(command) || WINDOWS_SYSTEM_COMMANDS.is_match(command) {
        return PlatformType::System;
    }

    PlatformType::Unknown
}
