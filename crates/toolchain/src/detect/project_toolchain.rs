use super::languages::{BUN, DENO, NODE};
use super::project_language::has_language_files;
use moon_common::Id;
use moon_config::{LanguageType, PlatformType};
use std::convert::TryFrom;
use std::path::Path;

/// Return a list of toolchains based on the provided ID.
pub fn get_project_toolchains(id: &Id) -> Vec<Id> {
    let mut toolchains = vec![id.to_owned()];

    // Since JS has multiple runtimes, we should inherit JS also
    if id == "bun" || id == "deno" || id == "node" {
        toolchains.push(Id::raw("javascript"));
    }
    // Otherwise check if we're a supported platform, if not, inherit system
    else if PlatformType::try_from(id.as_str()).is_err() {
        toolchains.push(Id::raw("system"));
    }

    toolchains
}

// Detect the correct toolchains based on the project's language
// and what config files exist in the project root.
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
