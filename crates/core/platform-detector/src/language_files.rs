use moon_bun_lang::BUN_INSTALL;
use moon_config::ProjectLanguage;
use moon_deno_lang::{DENO_DEPS, DVM};
use moon_go_lang::{G, GOENV, GOMOD, GVM};
use moon_lang::{DependencyManager, VersionManager};
use moon_node_lang::{NODE, NODENV, NPM, NVM, PNPM, YARN};
use moon_php_lang::{COMPOSER, PHPBREW, PHPENV};
use moon_python_lang::{PIP, PIPENV, PYENV};
use moon_ruby_lang::{BUNDLER, RBENV, RVM};
use moon_rust_lang::{CARGO, RUSTUP, RUSTUP_LEGACY};

fn extract_depman_files(depman: &DependencyManager, files: &mut Vec<String>) {
    files.push(depman.manifest.to_owned());
    files.push(depman.lockfile.to_owned());

    for file in depman.config_files {
        files.push(file.to_string());
    }
}

fn extract_verman_files(verman: &VersionManager, files: &mut Vec<String>) {
    files.push(verman.version_file.to_owned());
}

pub fn detect_language_files(language: &ProjectLanguage) -> Vec<String> {
    let mut files = vec![".prototools".to_owned()];

    match language {
        ProjectLanguage::Go => {
            extract_depman_files(&GOMOD, &mut files);
            extract_verman_files(&G, &mut files);
            extract_verman_files(&GVM, &mut files);
            extract_verman_files(&GOENV, &mut files);
        }
        ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
            // Bun
            extract_depman_files(&BUN_INSTALL, &mut files);

            // Deno
            extract_depman_files(&DENO_DEPS, &mut files);
            extract_verman_files(&DVM, &mut files);

            // Node
            extract_depman_files(&NPM, &mut files);
            extract_depman_files(&PNPM, &mut files);
            extract_depman_files(&YARN, &mut files);
            extract_verman_files(&NVM, &mut files);
            extract_verman_files(&NODENV, &mut files);

            for ext in NODE.file_exts {
                files.push(format!("postinstall.{ext}"));
            }
        }
        ProjectLanguage::Php => {
            extract_depman_files(&COMPOSER, &mut files);
            extract_verman_files(&PHPENV, &mut files);
            extract_verman_files(&PHPBREW, &mut files);
        }
        ProjectLanguage::Python => {
            extract_depman_files(&PIP, &mut files);
            extract_depman_files(&PIPENV, &mut files);
            extract_verman_files(&PYENV, &mut files);
        }
        ProjectLanguage::Ruby => {
            extract_depman_files(&BUNDLER, &mut files);
            extract_verman_files(&RVM, &mut files);
            extract_verman_files(&RBENV, &mut files);
        }
        ProjectLanguage::Rust => {
            extract_depman_files(&CARGO, &mut files);
            extract_verman_files(&RUSTUP, &mut files);
            extract_verman_files(&RUSTUP_LEGACY, &mut files);
        }
        ProjectLanguage::Bash
        | ProjectLanguage::Batch
        | ProjectLanguage::Unknown
        | ProjectLanguage::Other(_) => {}
    }

    files
}
