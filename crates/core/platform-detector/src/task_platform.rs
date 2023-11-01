use moon_config::PlatformType;
use moon_utils::regex::{self, UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use once_cell::sync::Lazy;

static BUN_COMMANDS: Lazy<regex::Regex> =
    Lazy::new(|| regex::create_regex("^(bun|bunx)$").unwrap());

static DENO_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| regex::create_regex("^(deno)$").unwrap());

static RUST_COMMANDS: Lazy<regex::Regex> =
    Lazy::new(|| regex::create_regex("^(rust-|rustc|rustdoc|rustfmt|rustup|cargo)").unwrap());

static NODE_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| {
    regex::create_regex("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap()
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

pub fn detect_task_platform(
    command: &str,
    // language: &LanguageType,
    enabled_platforms: &[PlatformType],
) -> PlatformType {
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

    if UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command) {
        return PlatformType::System;
    }

    // Default to the platform of the project's language
    // let platform: PlatformType = language.clone().into();

    // if platform.is_unknown() {
    //     return PlatformType::System;
    // }

    // use_platform_if_enabled(platform, toolchain_config)

    PlatformType::Unknown
}
