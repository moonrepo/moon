use clap::Args;
use moon_app_components::ExtensionRegistry;
use moon_plugin::Id;
use proto_core::PluginLocator;
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct ExtArgs {
    #[arg(help = "ID of the extension to execute")]
    id: Id,

    // Passthrough args (after --)
    #[arg(last = true, help = "Arguments to pass through to the extension")]
    pub passthrough: Vec<String>,
}

#[system]
pub async fn ext(args: ArgsRef<ExtArgs>, extensions: ResourceRef<ExtensionRegistry>) {
    // Load the plugin
    extensions
        .load(
            &args.id,
            PluginLocator::SourceUrl {
                url: "https".into(),
            },
        )
        .await?;

    // Execute the plugin
    extensions.perform_sync(&args.id, |plugin| plugin.execute())?;
}
