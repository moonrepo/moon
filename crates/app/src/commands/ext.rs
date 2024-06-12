use crate::app_error::AppError;
use crate::session::CliSession;
use clap::Args;
use moon_common::Id;
use moon_plugin::{serialize_config, PluginId};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ExtArgs {
    #[arg(required = true, help = "ID of the extension to execute")]
    id: Id,

    // Passthrough args (after --)
    #[arg(last = true, help = "Arguments to pass through to the extension")]
    pub passthrough: Vec<String>,
}

#[instrument(skip_all)]
pub async fn ext(session: CliSession, args: ExtArgs) -> AppResult {
    let Some(config) = session.workspace_config.extensions.get(&args.id) else {
        return Err(AppError::UnknownExtension { id: args.id }.into());
    };

    let id = PluginId::raw(&args.id);
    let extensions = session.get_extension_registry()?;

    // Load and configure the plugin
    extensions
        .load_with_config(&id, config.get_plugin_locator(), move |manifest| {
            manifest.config.insert(
                "moon_extension_config".to_owned(),
                serialize_config(config.config.iter())?,
            );

            Ok(())
        })
        .await?;

    // Execute the plugin
    extensions.perform_sync(&id, |plugin, context| {
        plugin.execute(args.passthrough.clone(), context)
    })?;

    Ok(())
}
