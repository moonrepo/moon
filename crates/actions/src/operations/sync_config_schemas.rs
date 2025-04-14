use moon_app_context::AppContext;
use moon_common::color;
use moon_config::Version;
use moon_config_schema::json_schemas::generate_json_schemas;
use moon_hash::hash_content;
use tracing::{instrument, warn};

hash_content!(
    pub struct ConfigSchemaHash<'cfg> {
        pub files_exist: bool,
        pub moon_version: &'cfg Version,
    }
);

#[instrument(skip_all)]
pub async fn sync_config_schemas(app_context: &AppContext, force: bool) -> miette::Result<bool> {
    let out_dir = app_context.cache_engine.cache_dir.join("schemas");

    if let Err(error) = if force {
        generate_json_schemas(
            out_dir,
            app_context
                .toolchain_registry
                .define_toolchain_config_all()
                .await?,
        )
        .map(|_| true)
    } else {
        let files = vec![
            out_dir.join("project.json"),
            out_dir.join("tasks.json"),
            out_dir.join("template-frontmatter.json"),
            out_dir.join("template.json"),
            out_dir.join("toolchain.json"),
            out_dir.join("workspace.json"),
        ];

        app_context
            .cache_engine
            .execute_if_changed(
                "configSchemas.json",
                ConfigSchemaHash {
                    files_exist: files.into_iter().all(|file| file.exists()),
                    moon_version: &app_context.cli_version,
                },
                || async {
                    generate_json_schemas(
                        out_dir,
                        app_context
                            .toolchain_registry
                            .define_toolchain_config_all()
                            .await?,
                    )
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
