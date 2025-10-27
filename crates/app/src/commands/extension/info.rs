use crate::app_error::AppError;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::{Id, is_test_env};
use moon_config::ExtensionsConfig;
use moon_console::ui::*;
use moon_extension_plugin::ExtensionPlugin;
use proto_core::PluginLocator;
use schematic::SchemaType;
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
            "execute_extension",
            "sync_project",
            "sync_workspace",
            "extend_project_graph",
            "extend_task_command",
            "extend_task_script",
        ],
        &["register_extension"],
    )
    .await;

    let config_schema = if extension.has_func("define_extension_config").await {
        extension
            .define_extension_config()
            .await
            .map(|output| match output.schema.ty {
                SchemaType::Struct(inner) => Some(inner),
                _ => None,
            })?
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

        #(config_schema.map(|schema| {
            element! {
                Section(title: "Configuration") {
                    Stack(gap: 1) {
                        #(schema.fields.into_iter().map(|(field, setting)| {
                            let mut flags = vec![];

                            if setting.deprecated.is_some() {
                                flags.push("deprecated");
                            }

                            if !setting.optional {
                                flags.push("required");
                            }

                            element! {
                                Stack {
                                    View {
                                        StyledText(
                                            content: format!(
                                                "<property>{}</property><muted>:</muted> {} {}",
                                                field,
                                                setting.schema,
                                                if flags.is_empty() {
                                                    "".to_string()
                                                } else {
                                                    format!(
                                                        "<muted>({})</muted>",
                                                        flags.join(", ")
                                                    )
                                                }
                                            )
                                        )
                                    }
                                    #(setting.comment.as_ref().map(|comment| {
                                        element! {
                                            View {
                                                StyledText(
                                                    content: comment,
                                                    style: Style::MutedLight
                                                )
                                            }
                                        }
                                    }))
                                }
                            }.into_any()
                        }))
                    }
                }
            }.into_any()
        }))

        Section(title: "APIs") {
            #(apis.into_iter().map(|(api, implemented, required)| {
                element! {
                    List {
                        ListItem(
                            bullet: if implemented {
                                "🟢"
                            } else {
                                "⚫️"
                            }.to_owned()
                        ) {
                            StyledText(
                                content: if required {
                                    format!("{api} <muted>(required)</muted>")
                                } else {
                                    api
                                },
                                style: Style::MutedLight
                            )
                        }
                    }
                }
            }))
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

    list
}
