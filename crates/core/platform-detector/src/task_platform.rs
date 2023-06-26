use moon_config::{PlatformType, ToolchainConfig};
use moon_utils::regex::{self, UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use once_cell::sync::Lazy;

static DENO_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| regex::create_regex("^(deno)$").unwrap());

static RUST_COMMANDS: Lazy<regex::Regex> =
    Lazy::new(|| regex::create_regex("^(rust-|rustc|rustdoc|rustfmt|rustup|cargo)").unwrap());

static NODE_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| {
    regex::create_regex("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap()
});

fn use_platform_if_enabled(
    platform: PlatformType,
    toolchain_config: &ToolchainConfig,
) -> PlatformType {
    match platform {
        PlatformType::Deno if toolchain_config.deno.is_some() => return platform,
        PlatformType::Node if toolchain_config.node.is_some() => return platform,
        PlatformType::Rust if toolchain_config.rust.is_some() => return platform,
        _ => {}
    };

    PlatformType::System
}

pub fn detect_task_platform(
    command: &str,
    // language: &LanguageType,
    toolchain_config: &ToolchainConfig,
) -> PlatformType {
    if DENO_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Deno, toolchain_config);
    }

    if NODE_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Node, toolchain_config);
    }

    if RUST_COMMANDS.is_match(command) {
        return use_platform_if_enabled(PlatformType::Rust, toolchain_config);
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
