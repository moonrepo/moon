use super::languages::{BUN, DENO, NODE};
use super::project_language::has_language_files;
use moon_common::Id;
use moon_config::LanguageType;
use std::path::Path;

pub fn detect_project_toolchains(
    root: &Path,
    language: &LanguageType,
    enabled_toolchains: &[Id],
) -> Vec<Id> {
    let mut toolchains = vec![];

    match language {
        LanguageType::JavaScript | LanguageType::TypeScript => {
            let runtimes = vec![
                (Id::raw("deno"), DENO),
                (Id::raw("bun"), BUN),
                (Id::raw("node"), NODE),
            ];

            for (id, files) in runtimes {
                if enabled_toolchains.contains(&id) && has_language_files(root, files) {
                    toolchains.push(id);
                    toolchains.extend(language.get_toolchain_ids());
                    break;
                }
            }
        }
        other => {
            let id = Id::raw(other.to_string());

            if enabled_toolchains.contains(&id) {
                toolchains.push(id);
            }
        }
    };

    toolchains
}
