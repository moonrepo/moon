use std::path::Path;

use moon_app_context::AppContext;
use moon_common::color;
use moon_config::Version;
use moon_config_schema::{json_schemas::generate_json_schemas, pkl_schemas::generate_pkl_schemas};
use moon_hash::fingerprint;
use moon_process::find_command_on_path;
use tracing::{instrument, warn};

fingerprint!(
    pub struct ConfigSchemaFingerprint<'cfg> {
        pub files_exist: bool,
        pub moon_version: &'cfg Version,
    }
);

#[instrument(skip_all)]
pub fn sync_pkl_schemas(cache_dir: &Path) -> miette::Result<()> {
    let pkl_dir = cache_dir.join("schemas/pkl");

    if !pkl_dir.exists() && find_command_on_path("pkl").is_some() {
        generate_pkl_schemas(pkl_dir)?;
    }

    Ok(())
}

#[instrument(skip_all)]
pub async fn sync_config_schemas(app_context: &AppContext, force: bool) -> miette::Result<bool> {
    let out_dir = app_context.cache_engine.cache_dir.join("schemas");

    if let Err(error) = if force {
        sync_pkl_schemas(&app_context.cache_engine.cache_dir)?;

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
            out_dir.join("extensions.json"),
            out_dir.join("project.json"),
            out_dir.join("tasks.json"),
            out_dir.join("template-frontmatter.json"),
            out_dir.join("template.json"),
            out_dir.join("toolchains.json"),
            out_dir.join("workspace.json"),
        ];

        app_context
            .cache_engine
            .execute_if_changed(
                "config-schemas",
                ConfigSchemaFingerprint {
                    files_exist: files.into_iter().all(|file| file.exists()),
                    moon_version: &app_context.cli_version,
                },
                async |_| {
                    sync_pkl_schemas(&app_context.cache_engine.cache_dir)?;

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
            .map(|result| result.unwrap_or_default())
    } {
        warn!(
            "Failed to generate schemas for configuration: {}",
            color::muted_light(error.to_string())
        );

        return Ok(false);
    }

    Ok(true)
}
