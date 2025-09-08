use crate::app_error::AppError;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::{Id, is_test_env};
use moon_config::ToolchainConfig;
use moon_console::ui::*;
use moon_toolchain_plugin::ToolchainPlugin;
use proto_core::PluginLocator;
use schematic::SchemaType;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainInfoArgs {
    #[arg(help = "ID of the toolchain to inspect")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the toolchain")]
    plugin: Option<PluginLocator>,
}

#[instrument(skip_all)]
pub async fn info(session: MoonSession, args: ToolchainInfoArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .or_else(|| ToolchainConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    let toolchain = session
        .get_toolchain_registry()
        .await?
        .load_without_config(&args.id, &locator)
        .await?;

    let tier1_apis = collect_tier_apis(
        &toolchain,
        &[
            "register_toolchain",
            "define_toolchain_config",
            "initialize_toolchain",
            "detect_version_files",
            "parse_version_file",
            "define_docker_metadata",
            "scaffold_docker",
            "prune_docker",
            "sync_project",
            "sync_workspace",
        ],
        &["register_toolchain"],
    )
    .await;

    let tier2_apis = collect_tier_apis(
        &toolchain,
        &[
            "extend_project_graph",
            "extend_task_command",
            "extend_task_script",
            "define_requirements",
            "locate_dependencies_root",
            "install_dependencies",
            "hash_task_contents",
            "parse_lock",
            "parse_manifest",
            "setup_environment",
        ],
        &[],
    )
    .await;

    let tier3_apis = collect_tier_apis(
        &toolchain,
        &[
            "register_tool",
            "load_versions",
            "resolve_version",
            "download_prebuilt",
            "unpack_archive",
            "locate_executables",
            "setup_toolchain",
            "teardown_toolchain",
        ],
        &["register_tool", "download_prebuilt", "locate_executables"],
    )
    .await;

    let config_schema = if toolchain.has_func("define_toolchain_config").await {
        toolchain
            .define_toolchain_config()
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
            Section(title: "Toolchain") {
                #(toolchain.metadata.description.as_ref().map(|description| {
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
                            content: toolchain.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: "Name",
                    content: toolchain.metadata.name.clone(),
                )
                #((!is_test_env()).then(|| {
                    element! {
                        Entry(
                            name: "Version",
                            value: element! {
                                StyledText(
                                    content: toolchain.metadata.plugin_version.to_string(),
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

            Section(title: "Tier 1 - Usage detection") {
                #((!toolchain.metadata.config_file_globs.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Config files",
                            value: element! {
                                StyledText(
                                    content: toolchain.metadata.config_file_globs
                                        .iter()
                                        .map(|file| format!("<file>{file}</file>"))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    style: Style::MutedLight
                                )
                            }.into_any()
                        )
                    }
                }))
                #((!toolchain.metadata.exe_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Executable names",
                            value: element! {
                                StyledText(
                                    content: toolchain.metadata.exe_names
                                        .iter()
                                        .map(|exe| format!("<shell>{exe}</shell>"))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    style: Style::MutedLight
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(name: "APIs") {
                    #(tier1_apis.into_iter().map(|(api, implemented, required)| {
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

            Section(title: "Tier 2 - Ecosystem integration") {
                #((!toolchain.metadata.manifest_file_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Manifest files",
                            value: element! {
                                StyledText(
                                    content: toolchain.metadata.manifest_file_names
                                        .iter()
                                        .map(|file| format!("<file>{file}</file>"))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    style: Style::MutedLight
                                )
                            }.into_any()
                        )
                    }
                }))
                #((!toolchain.metadata.lock_file_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Lock files",
                            value: element! {
                                StyledText(
                                    content: toolchain.metadata.lock_file_names
                                        .iter()
                                        .map(|file| format!("<file>{file}</file>"))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    style: Style::MutedLight
                                )
                            }.into_any()
                        )
                    }
                }))
                #(toolchain.metadata.vendor_dir_name.as_ref().map(|vendor_dir_name| {
                    element! {
                        Entry(
                            name: "Vendor directory",
                            value: element! {
                                StyledText(
                                    content: vendor_dir_name,
                                    style: Style::File
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(name: "APIs") {
                    #(tier2_apis.into_iter().map(|(api, implemented, required)| {
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

            Section(title: "Tier 3 - Tool management") {
                Entry(name: "APIs") {
                    #(tier3_apis.into_iter().map(|(api, implemented, required)| {
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
        }
    })?;

    Ok(None)
}

async fn collect_tier_apis(
    toolchain: &ToolchainPlugin,
    apis: &[&str],
    required: &[&str],
) -> Vec<(String, bool, bool)> {
    let mut list = vec![];

    for api in apis {
        list.push((
            api.to_string(),
            toolchain.has_func(api).await,
            required.contains(api),
        ));
    }

    list
}
