use super::languages::{BUN, DENO, NODE};
use super::project_language::has_language_files;
use moon_common::Id;
use moon_config::LanguageType;
use std::path::Path;

pub fn detect_project_toolchains(
    workspace_root: &Path,
    project_root: &Path,
    language: &LanguageType,
) -> Vec<Id> {
    let mut toolchains = vec![];

    if matches!(
        language,
        LanguageType::JavaScript | LanguageType::TypeScript
    ) {
        let runtimes = vec![
            (Id::raw("deno"), DENO),
            (Id::raw("bun"), BUN),
            (Id::raw("node"), NODE),
        ];

        for (id, files) in runtimes {
            // Detect in project first
            if has_language_files(project_root, files) {
                toolchains.push(id);
                break;
            }

            // Then in workspace
            if has_language_files(workspace_root, files) {
                toolchains.push(id);
                break;
            }
        }
    }

    toolchains.extend(language.get_toolchain_ids());
    toolchains
}
