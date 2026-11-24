use crate::app_error::AppError;
use crate::helpers::append_plugin_to_config_file;
use crate::prompts::*;
use crate::session::{MoonSession, SessionResult};
use clap::Args;
use iocraft::prelude::element;
use moon_common::{Id, IdExt};
use moon_config::PartialToolchainPluginConfig;
use moon_config_loader::ToolchainsConfigExt;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_pdk_api::{DefineRequirementsInput, InitializeToolchainInput, PromptType, SettingPrompt};
use moon_toolchain_plugin::{ToolchainPlugin, ToolchainRegistry};
use proto_core::PluginLocator;
use rustc_hash::FxHashSet;
use starbase_utils::json::JsonValue;
use std::collections::VecDeque;
use std::sync::Arc;
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

/// A toolchain that another one needs, which is added with it.
struct NeededToolchain {
    config: PartialToolchainPluginConfig,
    needed_by: Id,
    toolchain: Arc<ToolchainPlugin>,
}

#[instrument(skip(session))]
pub async fn add(session: MoonSession, args: ToolchainAddArgs) -> SessionResult {
    let Some(locator) = args
        .plugin
        .clone()
        .or_else(|| ToolchainsConfigExt::locate_plugin(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    // Load toolchain
    let toolchain_registry = session.get_toolchain_registry().await?;
    let toolchain = toolchain_registry.do_load(&args.id, &locator).await?;

    // Generate config
    let config =
        create_config_from_prompts(&session, &args, &toolchain_registry, &toolchain, None).await?;

    // Generate the configs of the toolchains it needs, which aren't configured
    let needed =
        create_needed_configs(&session, &args, &toolchain_registry, &toolchain, &config).await?;

    // Update toolchain file, with the toolchains it needs first, as each was
    // found by a toolchain before it
    for needed in needed.into_iter().rev() {
        add_to_config_file(
            &session,
            &needed.toolchain.id,
            Some(&needed.needed_by),
            needed.config,
        )?;
    }

    add_to_config_file(&session, &toolchain.id, None, config)?;

    Ok(None)
}

fn add_to_config_file(
    session: &MoonSession,
    id: &Id,
    needed_by: Option<&Id>,
    config: PartialToolchainPluginConfig,
) -> miette::Result<()> {
    let config_path =
        append_plugin_to_config_file(id, session.config_loader.get_toolchains_files(), config)?;
    let config_file = config_path
        .strip_prefix(&session.workspace_root)
        .unwrap()
        .display();

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: match needed_by {
                        Some(needed_by) => format!(
                            "Added toolchain <id>{id}</id> (needed by <id>{needed_by}</id>) to <file>{config_file}</file>!",
                        ),
                        None => format!(
                            "Added toolchain <id>{id}</id> to <file>{config_file}</file>!",
                        ),
                    }
                )
            }
        }
    })?;

    Ok(())
}

/// Generate the config of each toolchain that the toolchain needs, which
/// isn't configured: the toolchain that it inherits its dependencies root
/// from, and each that it requires, and those they need in turn. Like Bun
/// and Node.js, which need JavaScript to install their dependencies.
#[instrument(skip_all)]
async fn create_needed_configs(
    session: &MoonSession,
    args: &ToolchainAddArgs,
    toolchain_registry: &ToolchainRegistry,
    toolchain: &Arc<ToolchainPlugin>,
    config: &PartialToolchainPluginConfig,
) -> miette::Result<Vec<NeededToolchain>> {
    let mut found = FxHashSet::from_iter([toolchain.id.clone()]);
    let mut queue = VecDeque::from([(Arc::clone(toolchain), to_toolchain_config(config))]);
    let mut needed = vec![];

    while let Some((needing, needing_config)) = queue.pop_front() {
        let output = toolchain_registry
            .define_requirements_for(&needing, |_| DefineRequirementsInput {
                context: toolchain_registry.create_context(),
                toolchain_config: needing_config.clone(),
            })
            .await?;

        let ids = output.inherits_root_from.into_iter().chain(
            output
                .requires
                .into_iter()
                .map(|requirement| requirement.id),
        );

        for id in ids {
            if !found.insert(id.clone())
                || session.toolchains_config.get_plugin_config(&id).is_some()
            {
                continue;
            }

            let Some(locator) = ToolchainsConfigExt::locate_plugin(&id) else {
                session.console.render(element! {
                    Container {
                        Notice(variant: Variant::Caution) {
                            StyledText(
                                content: format!(
                                    "Toolchain <id>{id}</id> is needed by <id>{}</id>, but isn't built-in, so it wasn't added. Add it with <shell>moon toolchain add {id}</shell> and its plugin locator.",
                                    needing.id,
                                )
                            )
                        }
                    }
                })?;

                continue;
            };

            let needed_toolchain = toolchain_registry.do_load(&id, &locator).await?;
            let needed_config = create_config_from_prompts(
                session,
                args,
                toolchain_registry,
                &needed_toolchain,
                Some(&needing.id),
            )
            .await?;

            queue.push_back((
                Arc::clone(&needed_toolchain),
                to_toolchain_config(&needed_config),
            ));

            needed.push(NeededToolchain {
                config: needed_config,
                needed_by: needing.id.clone(),
                toolchain: needed_toolchain,
            });
        }
    }

    Ok(needed)
}

/// The config as a plugin receives it, like `ToolchainPluginConfig::to_json`.
fn to_toolchain_config(config: &PartialToolchainPluginConfig) -> JsonValue {
    let mut data = JsonValue::Object(
        config
            .config
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect(),
    );

    if let Some(version) = &config.version {
        data["version"] = JsonValue::String(version.to_string());
    }

    data
}

/// Default each select prompt that has the toolchain as an option to it, as
/// that's what the prompted toolchain was added for. Like the package manager
/// of JavaScript, when it's added for Bun.
fn default_prompts_to_toolchain(prompts: &mut [SettingPrompt], id: &Id) {
    let (stable_id, unstable_id) = Id::stable_and_unstable(id);

    for prompt in prompts {
        if let PromptType::Select {
            default_index,
            options,
        } = &mut prompt.ty
            && let Some(index) = options.iter().position(|option| {
                option.as_str().is_some_and(|option| {
                    option == stable_id.as_str() || option == unstable_id.as_str()
                })
            })
        {
            *default_index = index;
        }

        default_prompts_to_toolchain(&mut prompt.prompts, id);
    }
}

/// Generate the config of a toolchain from its prompts. When it's added as
/// another toolchain needs it, `needed_by` is that toolchain.
#[instrument(skip_all)]
pub async fn create_config_from_prompts(
    session: &MoonSession,
    args: &ToolchainAddArgs,
    toolchain_registry: &ToolchainRegistry,
    toolchain: &ToolchainPlugin,
    needed_by: Option<&Id>,
) -> miette::Result<PartialToolchainPluginConfig> {
    let mut config = PartialToolchainPluginConfig::default();

    // Gather built-in settings, where only the toolchain being added
    // has a locator, as the ones it needs are built-in
    if args.plugin.is_some() && needed_by.is_none() {
        config.plugin = Some(toolchain.locator.clone());
    }

    if toolchain.supports_tier_3().await {
        config.version = toolchain.detect_version(&session.working_dir).await?;

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

    // No instructions, so return early
    if !toolchain.has_func("initialize_toolchain").await {
        return Ok(config);
    }

    // Extract information from the plugin
    let mut output = toolchain
        .initialize_toolchain(InitializeToolchainInput {
            context: toolchain_registry.create_context(),
            toolchain_config: toolchain_registry.create_config(&toolchain.id),
        })
        .await?;

    if let Some(needed_by) = needed_by {
        default_prompts_to_toolchain(&mut output.prompts, needed_by);
    }

    let settings = evaluate_plugin_initialize_prompts(
        &session.console,
        &toolchain.metadata.name,
        "Toolchain",
        "https://moonrepo.dev/docs/concepts/toolchain",
        output,
        args.minimal,
        args.yes,
    )
    .await?;

    config.config.get_or_insert_default().extend(settings);

    Ok(config)
}
