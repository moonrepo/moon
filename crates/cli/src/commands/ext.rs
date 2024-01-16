use clap::Args;
use miette::miette;
use moon_app_components::ExtensionRegistry;
use moon_common::color;
use moon_plugin::{serialize_config, Id};
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
        return Err(miette!(
            code = "plugin::missing_extension",
            "The extension {} does not exist. Configure an {} entry in {} and try again.",
            color::id(&args.id),
            color::property("extensions"),
            color::file(".moon/workspace.yml"),
        ));
    };

    // Load and configure the plugin
    extensions
        .load_with_config(&args.id, config.plugin.as_ref().unwrap(), move |manifest| {
            manifest.config.insert(
                "moon_extension_config".to_owned(),
                serialize_config(&config.config)?,
            );

            Ok(())
        })
        .await?;

    // Execute the plugin
    extensions.perform_sync(&args.id, |plugin| plugin.execute())?;
}
