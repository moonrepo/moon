use miette::IntoDiagnostic;
use moon_logger::debug;
use moon_workspace::Workspace;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{fs, json};

const LOG_TARGET: &str = "moon:query:hash";

pub async fn query_hash(workspace: &Workspace, hash: &str) -> AppResult<(String, String)> {
    debug!(
        target: LOG_TARGET,
        "Querying for hash manifest with {}",
        color::hash(hash)
    );

    for file in fs::read_dir(&workspace.hash_engine.hashes_dir)? {
        let path = file.path();
        let name = fs::file_name(&path).replace(".json", "");

        if hash == name || name.starts_with(hash) {
            debug!(
                target: LOG_TARGET,
                "Found hash manifest {} for {}",
                color::id(&name),
                color::hash(hash)
            );

            // Our cache is non-pretty, but we wan't to output as pretty,
            // so we need to manually convert it here!
            let data: json::JsonValue = json::read_file(path)?;

            return Ok((name, json::to_string_pretty(&data).into_diagnostic()?));
        }
    }

    Err(miette::miette!(
        "Unable to find a hash manifest for {}!",
        color::hash(hash)
    ))
}
