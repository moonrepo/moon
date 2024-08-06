use crate::app_error::AppError;
use crate::session::CliSession;
use clap::Args;
use moon_common::Id;
use moon_plugin::PluginId;
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
    if !session.workspace_config.extensions.contains_key(&args.id) {
        return Err(AppError::UnknownExtension { id: args.id }.into());
    }

    let id = PluginId::raw(&args.id);
    let extensions = session.get_extension_registry()?;

    // Load the plugin
    extensions.load(&id).await?;

    // Execute the plugin
    let plugin = extensions.get(&id).await?;

    plugin
        .execute(args.passthrough, extensions.create_context())
        .await?;

    // let passthrough_args = args.passthrough.clone();

    // extensions
    //     .perform(&id, |plugin, context| async move {
    //         plugin.execute(passthrough_args, context).await
    //     })
    //     .await?;

    Ok(())
}
