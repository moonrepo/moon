use crate::app_error::AppError;
use crate::components::{ApiList, ConfigSettings};
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::{Id, is_test_env};
use moon_config::ExtensionsConfig;
use moon_console::ui::*;
use moon_extension_plugin::ExtensionPlugin;
use proto_core::PluginLocator;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ExtensionInfoArgs {
    #[arg(help = "Extension ID to inspect")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the extension")]
    plugin: Option<PluginLocator>,
}

#[instrument(skip(session))]
pub async fn info(session: MoonSession, args: ExtensionInfoArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .or_else(|| ExtensionsConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    let extension = session
        .get_extension_registry()
        .await?
        .load_without_config(&args.id, &locator)
        .await?;

    let apis = collect_apis(
        &extension,
        &[
            "register_extension",
            "define_extension_config",
            "initialize_extension",
            "execute_extension",
            "sync_project",
            "sync_workspace",
            "extend_command",
            "extend_project_graph",
            "extend_task_command",
            "extend_task_script",
        ],
        &["register_extension"],
    )
    .await;

    let config_schema = if extension.has_func("define_extension_config").await {
        Some(extension.define_extension_config().await?.schema)
    } else {
        None
    };

    session.console.render(element! {
        Container {
            Section(title: "Extension") {
                #(extension.metadata.description.as_ref().map(|description| {
                    element! {
                        View(margin_bottom: 1) {
                            StyledText(
                                content: description
                            )
                        }
                    }
                }))
                Entry(
                    name: "ID",
                    value: element! {
                        StyledText(
                            content: extension.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: "Title",
                    content: extension.metadata.name.clone(),
                )
                #((!is_test_env()).then(|| {
                    element! {
                        Entry(
                            name: "Version",
                            value: element! {
                                StyledText(
                                    content: extension.metadata.plugin_version.to_string(),
                                    style: Style::Hash
                                )
                            }.into_any()
                        )
                    }
                }))
            }

            #(config_schema.as_ref().map(|schema| {
                element! {
                    Section(title: "Configuration") {
                        ConfigSettings(schema: Some(schema))
                    }
                }.into_any()
            }))

            Section(title: "APIs") {
                ApiList(apis)
            }
        }
    })?;

    Ok(None)
}

async fn collect_apis(
    extension: &ExtensionPlugin,
    apis: &[&str],
    required: &[&str],
) -> Vec<(String, bool, bool)> {
    let mut list = vec![];

    for api in apis {
        list.push((
            api.to_string(),
            extension.has_func(api).await,
            required.contains(api),
        ));
    }

    list.sort_by(|a, d| a.0.cmp(&d.0));

    list
}
