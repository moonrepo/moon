use crate::app_error::AppError;
use crate::helpers::append_plugin_to_config_file;
use crate::prompts::*;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_common::Id;
use moon_config::{PartialToolchainPluginConfig, ToolchainsConfig};
use moon_console::ui::{Container, Entry, Notice, Section, Style, StyledText, Variant};
use moon_pdk_api::InitializeToolchainInput;
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use proto_core::PluginLocator;
use starbase::AppResult;
use starbase_utils::json::JsonMap;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainAddArgs {
    #[arg(help = "Unique ID of the toolchain to add")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the toolchain")]
    plugin: Option<PluginLocator>,

    #[arg(long, help = "Add with minimal configuration and prompts")]
    minimal: bool,

    #[arg(long, help = "Skip prompts and use default values")]
    yes: bool,
}

#[instrument(skip(session))]
pub async fn add(session: MoonSession, args: ToolchainAddArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .clone()
        .or_else(|| ToolchainsConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    // Load toolchain
    let toolchain_registry = session.get_toolchain_registry().await?;
    let toolchain = toolchain_registry
        .load_without_config(&args.id, &locator)
        .await?;

    // Generate config
    let config =
        create_config_from_prompts(&session, &args, &toolchain_registry, &toolchain).await?;

    // Update toolchain file
    let config_path = append_plugin_to_config_file(
        &toolchain.id,
        session
            .config_loader
            .get_toolchains_files(&session.workspace_root),
        config,
    )?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!(
                        "Added toolchain <id>{}</id> to <file>{}</file>!",
                        toolchain.id,
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
    args: &ToolchainAddArgs,
    toolchain_registry: &ToolchainRegistry,
    toolchain: &ToolchainPlugin,
) -> miette::Result<PartialToolchainPluginConfig> {
    let mut config = PartialToolchainPluginConfig::default();

    // No instructions, so render an empty block
    if !toolchain.has_func("initialize_toolchain").await {
        return Ok(config);
    }

    // Extract information from the plugin
    let output = toolchain
        .initialize_toolchain(InitializeToolchainInput {
            context: toolchain_registry.create_context(),
        })
        .await?;

    if !args.yes {
        session.console.render(element! {
            Container {
                Section(title: &toolchain.metadata.name) {
                    Entry(
                        name: "Toolchain",
                        value: element! {
                            StyledText(
                                content: "https://moonrepo.dev/docs/concepts/toolchain",
                                style: Style::Url
                            )
                        }.into_any()
                    )
                    #(output.docs_url.as_ref().map(|url| {
                        element! {
                            Entry(
                                name: "Handbook",
                                value: element! {
                                    StyledText(
                                        content: url,
                                        style: Style::Url
                                    )
                                }.into_any()
                            )
                        }
                    }))
                    #(output.config_url.as_ref().map(|url| {
                        element! {
                            Entry(
                                name: "Config",
                                value: element! {
                                    StyledText(
                                        content: url,
                                        style: Style::Url
                                    )
                                }.into_any()
                            )
                        }
                    }))
                }
            }
        })?;
    }

    // Gather built-in settings
    if args.plugin.is_some() {
        config.plugin = Some(toolchain.locator.clone());
    }

    if toolchain.supports_tier_3().await {
        if toolchain.has_func("detect_version_files").await
            && let Some(version) = toolchain.detect_version(&session.working_dir).await?
        {
            config.version = Some(version);
        }

        if config.version.is_none()
            && let Some(version) = render_version_prompt(
                &session.console,
                args.yes || args.minimal,
                &toolchain.metadata.name,
                || Ok(None),
            )
            .await?
        {
            config.version = Some(version);
        }
    }

    // Gather user settings via prompts
    let mut settings = JsonMap::from_iter(output.default_settings);

    evaluate_prompts(
        &session.console,
        &output.prompts,
        &mut settings,
        args.minimal,
        args.yes,
    )
    .await?;

    config.config.get_or_insert_default().extend(settings);

    Ok(config)
}
