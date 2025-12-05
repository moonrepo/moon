use crate::app_error::AppError;
use crate::helpers::append_plugin_to_config_file;
use crate::prompts::*;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_common::Id;
use moon_config::{ExtensionsConfig, PartialExtensionPluginConfig};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_extension_plugin::{ExtensionPlugin, ExtensionRegistry};
use moon_pdk_api::InitializeExtensionInput;
use proto_core::PluginLocator;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ExtensionAddArgs {
    #[arg(help = "Unique ID of the extension to add")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the extension")]
    plugin: Option<PluginLocator>,

    #[arg(long, help = "Add with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,
}

#[instrument(skip(session))]
pub async fn add(session: MoonSession, args: ExtensionAddArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .clone()
        .or_else(|| ExtensionsConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    // Load extension
    let extension_registry = session.get_extension_registry().await?;
    let extension = extension_registry
        .load_without_config(&args.id, &locator)
        .await?;

    // Generate config
    let config =
        create_config_from_prompts(&session, &args, &extension_registry, &extension).await?;

    // Update extension file
    let config_path = append_plugin_to_config_file(
        &extension.id,
        session.config_loader.get_extensions_files(),
        config,
    )?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Added extension <id>{}</id> to <file>{}</file>!",
                        extension.id,
                        config_path.strip_prefix(&session.workspace_root).unwrap().display(),
                    )
                )
            }
        }
    })?;

    Ok(None)
}

#[instrument(skip_all)]
pub async fn create_config_from_prompts(
    session: &MoonSession,
    args: &ExtensionAddArgs,
    extension_registry: &ExtensionRegistry,
    extension: &ExtensionPlugin,
) -> miette::Result<PartialExtensionPluginConfig> {
    let mut config = PartialExtensionPluginConfig::default();

    // Gather built-in settings
    if args.plugin.is_some() {
        config.plugin = Some(extension.locator.clone());
    }

    // No instructions, so return early
    if !extension.has_func("initialize_extension").await {
        return Ok(config);
    }

    // Extract information from the plugin
    let output = extension
        .initialize_extension(InitializeExtensionInput {
            context: extension_registry.create_context(),
        })
        .await?;

    let settings = evaluate_plugin_initialize_prompts(
        &session.console,
        &extension.metadata.name,
        "Extension",
        "https://moonrepo.dev/docs/concepts/extension",
        output,
        args.minimal,
        args.yes,
    )
    .await?;

    config.config.get_or_insert_default().extend(settings);

    Ok(config)
}
