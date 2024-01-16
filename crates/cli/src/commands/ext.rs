use clap::Args;
use moon_app_components::ExtensionRegistry;
use moon_plugin::Id;
use moon_workspace::Workspace;
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
pub async fn ext(
    args: ArgsRef<ExtArgs>,
    workspace: ResourceRef<Workspace>,
    extensions: ResourceRef<ExtensionRegistry>,
) {
    let Some(config) = workspace.config.extensions.get(&args.id) else {
        panic!(); // TODO
    };

    // Load the plugin
    extensions
        .load(&args.id, config.plugin.as_ref().unwrap())
        .await?;

    // Execute the plugin
    extensions.perform_sync(&args.id, |plugin| plugin.execute())?;
}
