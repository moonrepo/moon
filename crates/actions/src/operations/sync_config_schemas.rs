use moon_app_context::AppContext;
use moon_common::color;
use moon_config::Version;
use moon_config_schema::{Schema, json_schemas::generate_json_schemas};
use moon_hash::hash_content;
use rustc_hash::FxHashMap;
use tracing::{instrument, warn};

hash_content!(
    pub struct ConfigSchemaHash<'cfg> {
        moon_version: &'cfg Version,
    }
);

#[instrument(skip_all)]
pub async fn sync_config_schemas(app_context: &AppContext, force: bool) -> miette::Result<bool> {
    let out_dir = app_context.cache_engine.cache_dir.join("schemas");

    if let Err(error) = if force {
        generate_json_schemas(out_dir, get_toolchain_schemas(app_context).await?).map(|_| true)
    } else {
        app_context
            .cache_engine
            .execute_if_changed(
                "configSchemas.json",
                ConfigSchemaHash {
                    moon_version: &app_context.cli_version,
                },
                || async {
                    generate_json_schemas(out_dir, get_toolchain_schemas(app_context).await?)
                },
            )
            .await
    } {
        warn!(
            "Failed to generate schemas for configuration: {}",
            color::muted_light(error.to_string())
        );

        return Ok(false);
    }

    Ok(true)
}

async fn get_toolchain_schemas(
    app_context: &AppContext,
) -> miette::Result<FxHashMap<String, Schema>> {
    let mut toolchain_schemas = FxHashMap::default();

    for toolchain_id in app_context.toolchain_registry.get_plugin_ids() {
        let toolchain = app_context.toolchain_registry.load(toolchain_id).await?;

        if let Some(config_schema) = &toolchain.metadata.config_schema {
            toolchain_schemas.insert(toolchain_id.to_string(), config_schema.to_owned());
        }
    }

    Ok(toolchain_schemas)
}
