use moon_app_context::AppContext;
use moon_common::color;
use moon_config::Version;
use moon_config_schema::json_schemas::generate_json_schemas as generate;
use moon_hash::hash_content;
use tracing::{instrument, warn};

hash_content!(
    pub struct ConfigSchemaHash<'cfg> {
        moon_version: &'cfg Version,
    }
);

#[instrument(skip_all)]
pub async fn generate_json_schemas(app_context: &AppContext) -> miette::Result<()> {
    if let Err(error) = app_context
        .cache_engine
        .execute_if_changed(
            "configSchemas.json",
            ConfigSchemaHash {
                moon_version: &app_context.cli_version,
            },
            || async { generate(app_context.cache_engine.cache_dir.join("schemas")) },
        )
        .await
    {
        warn!(
            "Failed to generate schemas for configuration: {}",
            color::muted_light(error.to_string())
        );
    }

    Ok(())
}
