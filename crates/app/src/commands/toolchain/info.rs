use crate::app_error::AppError;
use crate::components::{ApiList, ConfigSettings};
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::{Id, is_test_env};
use moon_config::ToolchainsConfig;
use moon_console::ui::*;
use moon_toolchain_plugin::ToolchainPlugin;
use proto_core::PluginLocator;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainInfoArgs {
    #[arg(help = "Toolchain ID to inspect")]
    id: Id,

    #[arg(help = "Plugin locator string to find and load the toolchain")]
    plugin: Option<PluginLocator>,
}

#[instrument(skip(session))]
pub async fn info(session: MoonSession, args: ToolchainInfoArgs) -> AppResult {
    let Some(locator) = args
        .plugin
        .or_else(|| ToolchainsConfig::get_plugin_locator(&args.id))
    else {
        return Err(AppError::PluginLocatorRequired.into());
    };

    let toolchain = session
        .get_toolchain_registry()
        .await?
        .load_without_config(&args.id, &locator)
        .await?;
    let metadata = &toolchain.metadata;

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
            "extend_command",
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
        Some(toolchain.define_toolchain_config().await?.schema)
    } else {
        None
    };

    session.console.render(element! {
        Container {
            Section(title: "Toolchain") {
                #(metadata.description.as_ref().map(|description| {
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
                    name: "Title",
                    content: metadata.name.clone(),
                )
                #((!is_test_env()).then(|| {
                    element! {
                        Entry(
                            name: "Version",
                            value: element! {
                                StyledText(
                                    content: metadata.plugin_version.to_string(),
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

            Section(title: "Tier 1 - Usage detection") {
                #((!metadata.config_file_globs.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Config files",
                            value: element! {
                                StyledText(
                                    content: metadata.config_file_globs
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
                #((!metadata.exe_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Executable names",
                            value: element! {
                                StyledText(
                                    content: metadata.exe_names
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

                View(
                    margin_top: if metadata.config_file_globs.is_empty() && metadata.exe_names.is_empty() {
                        0
                    } else {
                        1
                    }
                ) {
                    ApiList(apis: tier1_apis)
                }
            }

            Section(title: "Tier 2 - Ecosystem integration") {
                #((!metadata.manifest_file_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Manifest files",
                            value: element! {
                                StyledText(
                                    content: metadata.manifest_file_names
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
                #((!metadata.lock_file_names.is_empty()).then(|| {
                    element! {
                        Entry(
                            name: "Lock files",
                            value: element! {
                                StyledText(
                                    content: metadata.lock_file_names
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
                #(metadata.vendor_dir_name.as_ref().map(|vendor_dir_name| {
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

                View(
                    margin_top: if metadata.manifest_file_names.is_empty() && metadata.lock_file_names.is_empty() && metadata.vendor_dir_name.is_none() {
                        0
                    } else {
                        1
                    }
                ) {
                    ApiList(apis: tier2_apis)
                }
            }

            Section(title: "Tier 3 - Tool management") {
                ApiList(apis: tier3_apis)
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

    list.sort_by(|a, d| a.0.cmp(&d.0));

    list
}
