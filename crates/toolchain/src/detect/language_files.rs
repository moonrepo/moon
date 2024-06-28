use super::languages::*;
use moon_config::LanguageType;

pub fn detect_language_files(language: &LanguageType) -> Vec<String> {
    let files = match language {
        LanguageType::Go => GO.to_vec(),
        LanguageType::JavaScript | LanguageType::TypeScript => {
            let mut files = vec![];
            files.extend(NODE);
            files.extend(DENO);
            files.extend(BUN);
            files.extend(TYPESCRIPT);
            files
        }
        LanguageType::Php => PHP.to_vec(),
        LanguageType::Python => PYTHON.to_vec(),
        LanguageType::Ruby => RUBY.to_vec(),
        LanguageType::Rust => RUST.to_vec(),
        LanguageType::Bash
        | LanguageType::Batch
        | LanguageType::Unknown
        | LanguageType::Other(_) => vec![],
    };

    files.into_iter().map(|file| file.to_owned()).collect()
}
