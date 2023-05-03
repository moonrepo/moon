use moon_config::{PlatformType, ProjectLanguage};
use moon_utils::regex::{self, UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use once_cell::sync::Lazy;

static DENO_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| regex::create_regex("^(deno)$").unwrap());

static RUST_COMMANDS: Lazy<regex::Regex> =
    Lazy::new(|| regex::create_regex("^(rust-|rustc|rustdoc|rustfmt|rustup|cargo)").unwrap());

static NODE_COMMANDS: Lazy<regex::Regex> = Lazy::new(|| {
    regex::create_regex("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap()
});

pub fn detect_task_platform(command: &str, language: &ProjectLanguage) -> PlatformType {
    if DENO_COMMANDS.is_match(command) {
        return PlatformType::Deno;
    }

    if NODE_COMMANDS.is_match(command) {
        return PlatformType::Node;
    }

    if RUST_COMMANDS.is_match(command) {
        return PlatformType::Rust;
    }

    if UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command) {
        return PlatformType::System;
    }

    // Default to the platform of the project's language
    let platform: PlatformType = language.clone().into();

    if platform.is_unknown() {
        return PlatformType::System;
    }

    platform
}
