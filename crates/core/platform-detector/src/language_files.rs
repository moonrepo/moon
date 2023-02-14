use moon_bun_lang::BUN_INSTALL;
use moon_config::ProjectLanguage;
use moon_deno_lang::DENO_DEPS;
use moon_go_lang::GOMOD;
use moon_lang::DependencyManager;
use moon_node_lang::{NODE, NPM, PNPM, YARN};
use moon_php_lang::COMPOSER;
use moon_python_lang::{PIP, PIPENV};
use moon_ruby_lang::BUNDLER;
use moon_rust_lang::CARGO;

fn extract_depman_files(depman: &DependencyManager, files: &mut Vec<String>) {
    files.push(depman.manifest.to_owned());
    files.push(depman.lockfile.to_owned());

    for file in depman.config_files {
        files.push(file.to_string());
    }
}

pub fn detect_language_files(language: ProjectLanguage) -> Vec<String> {
    let mut files = vec![];

    match language {
        ProjectLanguage::Go => {
            extract_depman_files(&GOMOD, &mut files);
        }
        ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => {
            // Bun
            extract_depman_files(&BUN_INSTALL, &mut files);

            // Deno
            extract_depman_files(&DENO_DEPS, &mut files);

            // Node
            extract_depman_files(&NPM, &mut files);
            extract_depman_files(&PNPM, &mut files);
            extract_depman_files(&YARN, &mut files);

            for ext in NODE.file_exts {
                files.push(format!("postinstall.{ext}"));
            }
        }
        ProjectLanguage::Php => {
            extract_depman_files(&COMPOSER, &mut files);
        }
        ProjectLanguage::Python => {
            extract_depman_files(&PIP, &mut files);
            extract_depman_files(&PIPENV, &mut files);
        }
        ProjectLanguage::Ruby => {
            extract_depman_files(&BUNDLER, &mut files);
        }
        ProjectLanguage::Rust => {
            extract_depman_files(&CARGO, &mut files);
        }
        ProjectLanguage::Bash | ProjectLanguage::Batch | ProjectLanguage::Unknown => {}
    }

    files
}
