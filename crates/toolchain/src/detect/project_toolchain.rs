use super::languages::{BUN, DENO, NODE};
use super::project_language::has_language_files;
use moon_common::Id;
use moon_config::LanguageType;
use std::path::Path;

pub fn detect_project_toolchains(
    workspace_root: &Path,
    project_root: &Path,
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
            let mut found = false;

            // Detect in project first
            for (id, files) in &runtimes {
                if has_language_files(project_root, files) {
                    toolchains.push(id.to_owned());
                    found = true;
                    break;
                }
            }

            // Then in workspace
            for (id, files) in runtimes {
                if !found && has_language_files(workspace_root, files) {
                    toolchains.push(id);
                    break;
                }
            }

            toolchains.extend(language.get_toolchain_ids());
        }
        other => {
            toolchains.push(Id::raw(other.to_string()));
        }
    };

    let mut toolchains = toolchains
        .into_iter()
        .filter(|id| enabled_toolchains.contains(id))
        .collect::<Vec<_>>();

    if toolchains.is_empty() {
        toolchains.push(Id::raw("system"));
    }

    toolchains
}
