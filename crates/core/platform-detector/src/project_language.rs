use moon_bun_lang::BUN_INSTALL;
use moon_config::ProjectLanguage;
use moon_deno_lang::{DENO_DEPS, DVM};
use moon_go_lang::{G, GOENV, GOMOD, GVM};
use moon_lang::{is_using_dependency_manager, is_using_version_manager};
use moon_node_lang::{NODENV, NPM, NVM, PNPM, YARN};
use moon_php_lang::{COMPOSER, PHPBREW, PHPENV};
use moon_python_lang::{PIP, PIPENV, PYENV};
use moon_ruby_lang::{BUNDLER, RBENV, RVM};
use moon_rust_lang::{CARGO, RUSTUP};
use std::path::Path;

pub fn detect_project_language(root: &Path) -> ProjectLanguage {
    // Go
    if is_using_dependency_manager(root, &GOMOD, true)
        || is_using_version_manager(root, &G)
        || is_using_version_manager(root, &GVM)
        || is_using_version_manager(root, &GOENV)
    {
        return ProjectLanguage::Go;
    }

    // PHP
    if is_using_dependency_manager(root, &COMPOSER, true)
        || is_using_version_manager(root, &PHPENV)
        || is_using_version_manager(root, &PHPBREW)
    {
        return ProjectLanguage::Php;
    }

    // Python
    if is_using_dependency_manager(root, &PIP, true)
        || is_using_dependency_manager(root, &PIPENV, true)
        || is_using_version_manager(root, &PYENV)
    {
        return ProjectLanguage::Python;
    }

    // Ruby
    if is_using_dependency_manager(root, &BUNDLER, true)
        || is_using_version_manager(root, &RVM)
        || is_using_version_manager(root, &RBENV)
    {
        return ProjectLanguage::Ruby;
    }

    // Rust
    if is_using_dependency_manager(root, &CARGO, true) || is_using_version_manager(root, &RUSTUP) {
        return ProjectLanguage::Rust;
    }

    // TypeScript (should take precedence over JavaScript)
    if root.join("tsconfig.json").exists()
        // Deno
        || is_using_dependency_manager(root, &DENO_DEPS, true)
        || is_using_version_manager(root, &DVM)
    {
        return ProjectLanguage::TypeScript;
    }

    // JavaScript (last since everyone uses it)
    if is_using_dependency_manager(root, &NPM, true)
        || is_using_dependency_manager(root, &PNPM, true)
        || is_using_dependency_manager(root, &YARN, true)
        || is_using_version_manager(root, &NVM)
        || is_using_version_manager(root, &NODENV)
        // Bun
        || is_using_dependency_manager(root, &BUN_INSTALL, true)
    {
        return ProjectLanguage::JavaScript;
    }

    ProjectLanguage::Unknown
}
