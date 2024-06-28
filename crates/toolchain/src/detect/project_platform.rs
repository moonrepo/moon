use super::languages::DENO;
use super::project_language::has_language_files;
use moon_config::{LanguageType, PlatformType};
use std::path::Path;

pub fn detect_project_platform(
    root: &Path,
    language: &LanguageType,
    enabled_platforms: &[PlatformType],
) -> PlatformType {
    match language {
        LanguageType::JavaScript | LanguageType::TypeScript => {
            if enabled_platforms.contains(&PlatformType::Deno) && has_language_files(root, DENO) {
                return PlatformType::Deno;
            }

            if enabled_platforms.contains(&PlatformType::Bun)
                && enabled_platforms.contains(&PlatformType::Node)
            {
                return PlatformType::Node;
            }

            if enabled_platforms.contains(&PlatformType::Bun) {
                PlatformType::Bun
            } else if enabled_platforms.contains(&PlatformType::Node) {
                PlatformType::Node
            } else {
                PlatformType::System
            }
        }
        LanguageType::Rust => {
            if enabled_platforms.contains(&PlatformType::Rust) {
                PlatformType::Rust
            } else {
                PlatformType::System
            }
        }
        _ => PlatformType::System,
    }
}
