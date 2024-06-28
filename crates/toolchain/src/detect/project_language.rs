use super::languages::*;
use moon_config::LanguageType;
use std::path::Path;

pub fn has_language_files(root: &Path, files: StaticStringList) -> bool {
    files.iter().any(|file| root.join(file).exists())
}

pub fn detect_project_language(root: &Path) -> LanguageType {
    if has_language_files(root, GO) {
        return LanguageType::Go;
    }

    if has_language_files(root, PHP) {
        return LanguageType::Php;
    }

    if has_language_files(root, PYTHON) {
        return LanguageType::Python;
    }

    if has_language_files(root, RUBY) {
        return LanguageType::Ruby;
    }

    if has_language_files(root, RUST) {
        return LanguageType::Rust;
    }

    // TypeScript (must run before JavaScript)
    if has_language_files(root, TYPESCRIPT) || has_language_files(root, DENO) {
        return LanguageType::TypeScript;
    }

    // JavaScript (last since everyone uses it)
    if has_language_files(root, NODE) || has_language_files(root, BUN) {
        return LanguageType::JavaScript;
    }

    LanguageType::Unknown
}
