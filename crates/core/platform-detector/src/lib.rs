use moon_config::{PlatformType, ProjectLanguage};
use moon_go_lang::{G, GOENV, GOMOD, GVM};
use moon_lang::{is_using_dependency_manager, is_using_version_manager};
use moon_node_lang::{NODENV, NPM, NVM, PNPM, YARN};
use moon_php_lang::{COMPOSER, PHPBREW, PHPENV};
use moon_python_lang::{PIP, PIPENV, PYENV};
use moon_ruby_lang::{BUNDLER, RBENV, RVM};
use moon_rust_lang::{CARGO, RUSTUP};
use moon_utils::regex::{UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use moon_utils::{lazy_static, regex::Regex};
use std::path::Path;

lazy_static! {
    pub static ref NODE_COMMANDS: Regex =
        Regex::new("^(node|nodejs|npm|npx|yarn|yarnpkg|pnpm|pnpx|corepack)$").unwrap();
}

pub fn detect_project_language(root: &Path) -> ProjectLanguage {
    // Go
    if is_using_dependency_manager(root, &GOMOD)
        || is_using_version_manager(root, &G)
        || is_using_version_manager(root, &GVM)
        || is_using_version_manager(root, &GOENV)
    {
        return ProjectLanguage::Go;
    }

    // PHP
    if is_using_dependency_manager(root, &COMPOSER)
        || is_using_version_manager(root, &PHPENV)
        || is_using_version_manager(root, &PHPBREW)
    {
        return ProjectLanguage::Php;
    }

    // Python
    if is_using_dependency_manager(root, &PIP)
        || is_using_dependency_manager(root, &PIPENV)
        || is_using_version_manager(root, &PYENV)
    {
        return ProjectLanguage::Python;
    }

    // Ruby
    if is_using_dependency_manager(root, &BUNDLER)
        || is_using_version_manager(root, &RVM)
        || is_using_version_manager(root, &RBENV)
    {
        return ProjectLanguage::Ruby;
    }

    // Rust
    if is_using_dependency_manager(root, &CARGO) || is_using_version_manager(root, &RUSTUP) {
        return ProjectLanguage::Rust;
    }

    // TypeScript (should take precedence over JavaScript)
    if root.join("tsconfig.json").exists()
        || root.join("deno.json").exists()
        || root.join("deno.jsonc").exists()
    {
        return ProjectLanguage::TypeScript;
    }

    // JavaScript (last since everyone uses it)
    if is_using_dependency_manager(root, &NPM)
        || is_using_dependency_manager(root, &PNPM)
        || is_using_dependency_manager(root, &YARN)
        || is_using_version_manager(root, &NVM)
        || is_using_version_manager(root, &NODENV)
    {
        return ProjectLanguage::JavaScript;
    }

    ProjectLanguage::Unknown
}

// TODO: Differentiate JS/TS between Node and Deno and Bun (in the future)
pub fn detect_task_platform(command: &str, language: ProjectLanguage) -> PlatformType {
    if NODE_COMMANDS.is_match(command) {
        return PlatformType::Node;
    }

    if UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command) {
        return PlatformType::System;
    }

    // Default to the platform of the project's language
    language.into()
}
