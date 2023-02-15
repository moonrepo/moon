use moon_config::{PlatformType, ProjectLanguage};
use moon_utils::regex::{UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use moon_utils::{lazy_static, regex::Regex};

lazy_static! {
    pub static ref DENO_COMMANDS: Regex = Regex::new("^(deno)$").unwrap();
    pub static ref NODE_COMMANDS: Regex =
        Regex::new("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap();
}

// TODO: Differentiate JS/TS between Node and Deno and Bun (in the future)
pub fn detect_task_platform(command: &str, language: ProjectLanguage) -> PlatformType {
    if DENO_COMMANDS.is_match(command) {
        return PlatformType::Deno;
    }

    if NODE_COMMANDS.is_match(command) {
        return PlatformType::Node;
    }

    if UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command) {
        return PlatformType::System;
    }

    // Default to the platform of the project's language
    let platform: PlatformType = language.into();

    if platform.is_unknown() {
        return PlatformType::System;
    }

    platform
}
