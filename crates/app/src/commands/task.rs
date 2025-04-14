use crate::app_error::AppError;
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_common::is_test_env;
use moon_console::ui::{
    Container, Entry, List, ListItem, Map, MapItem, Section, Style, StyledText,
};
use moon_task::{Target, TargetScope};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TaskArgs {
    #[arg(help = "Target of task to display")]
    target: Target,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn task(session: MoonSession, args: TaskArgs) -> AppResult {
    let TargetScope::Project(project_locator) = &args.target.scope else {
        return Err(AppError::ProjectIdRequired.into());
    };

    let workspace_graph = session.get_workspace_graph().await?;
    let project = workspace_graph.get_project(project_locator)?;
    let task = workspace_graph.get_task(&args.target)?;
    let console = &session.console;

    if args.json {
        console.out.write_line(json::format(&task, true)?)?;

        return Ok(None);
    }

    let mut modes = vec![];

    if task.is_local() {
        modes.push("local");
    }
    if task.is_internal() {
        modes.push("internal");
    }
    if task.is_interactive() {
        modes.push("interactive");
    }
    if task.is_persistent() {
        modes.push("persistent");
    }

    let mut inputs = vec![];
    inputs.extend(task.input_globs.iter().map(|i| i.to_string()));
    inputs.extend(task.input_files.iter().map(|i| i.to_string()));
    inputs.extend(task.input_env.iter().map(|i| format!("${i}")));
    inputs.sort();

    let mut outputs = vec![];
    outputs.extend(&task.output_globs);
    outputs.extend(&task.output_files);
    outputs.sort();

    let show_in_prod = !is_test_env();

    session.console.render(element! {
        Container {
            Section(title: "About") {
                #(task.description.as_ref().map(|desc| {
                    element! {
                        View(margin_bottom: 1) {
                            StyledText(
                                content: desc,
                            )
                        }
                    }
                }))

                Entry(
                    name: "Target",
                    value: element! {
                        StyledText(
                            content: task.target.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: "Project",
                    value: element! {
                        StyledText(
                            content: project.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: "Task",
                    value: element! {
                        StyledText(
                            content: task.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: if task.toolchains.len() == 1 {
                        "Toolchain"
                    } else {
                        "Toolchains"
                    },
                    content: task.toolchains.join(", "),
                )
                Entry(
                    name: "Type",
                    content: task.type_of.to_string(),
                )
                #(task.preset.as_ref().map(|preset| {
                    element! {
                        Entry(
                            name: "Preset",
                            content: preset.to_string(),
                        )
                    }
                }))
                #(if modes.is_empty() {
                    None
                } else {
                    Some(element! {
                        Entry(
                            name: if task.toolchains.len() == 1 {
                                "Mode"
                            } else {
                                "Modes"
                            },
                            content: modes.join(", "),
                        )
                    })
                })
            }

            Section(title: "Process") {
                Entry(
                    name: if task.script.is_some() {
                        "Script"
                    } else {
                        "Command"
                    },
                    value: element! {
                        StyledText(
                            content: task.get_command_line(),
                            style: Style::Shell
                        )
                    }.into_any()
                )

                #(show_in_prod.then(|| {
                    if task.options.shell.unwrap_or_default() {
                        element! {
                            Entry(
                                name: "Shell",
                                content: if cfg!(unix) {
                                    task.options.unix_shell.unwrap_or_default().to_string()
                                } else if cfg!(windows) {
                                    task.options.windows_shell.unwrap_or_default().to_string()
                                } else {
                                    "unknown".to_string()
                                }
                            )
                        }.into_any()
                    } else {
                        element!(View).into_any()
                    }
                }))
                Entry(
                    name: "Environment variables",
                    no_children: task.env.is_empty()
                ) {
                    Map {
                        #(task.env.iter().map(|(key, value)| {
                            element! {
                                MapItem(
                                    name: element! {
                                        StyledText(
                                            content: key,
                                            style: Style::Property
                                        )
                                    }.into_any(),
                                    value: element! {
                                        StyledText(
                                            content: value,
                                            style: Style::MutedLight
                                        )
                                    }.into_any(),
                                )
                            }
                        }))
                    }
                }
                Entry(
                    name: "Depends on",
                    no_children: task.deps.is_empty()
                ) {
                    List {
                        #(task.deps.iter().map(|dep| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: dep.target.to_string(),
                                        style: Style::Id
                                    )
                                }
                            }
                        }))
                    }
                }
                #(show_in_prod.then(|| {
                    element! {
                        Entry(
                            name: "Working directory",
                            value: element! {
                                StyledText(
                                    content: if task.options.run_from_workspace_root {
                                        &session.workspace_root
                                    } else {
                                        &project.root
                                    }.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: "Runs dependencies",
                    content: if task.options.run_deps_in_parallel {
                        "Concurrently"
                    } else {
                        "Serially"
                    }.to_string(),
                )
                Entry(
                    name: "Runs in CI",
                    content: if task.should_run_in_ci() {
                        "Yes"
                    } else {
                        "No"
                    }.to_string(),
                )
            }

            Section(title: "Configuration") {
                #(project.inherited
                    .as_ref()
                    .and_then(|inherited| inherited.task_layers.get(task.id.as_str()))
                    .map(|layers| {
                    element! {
                        Entry(
                            name: "Inherits from",
                            no_children: layers.is_empty()
                        ) {
                            List {
                                #(layers.iter().map(|layer| {
                                    element! {
                                        ListItem {
                                            StyledText(
                                                content: layer,
                                                style: Style::File
                                            )
                                        }
                                    }
                                }))
                            }
                        }
                    }
                }))
                Entry(
                    name: "Inputs",
                    no_children: inputs.is_empty()
                ) {
                    List {
                        #(inputs.iter().map(|input| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: input,
                                        style: if input.starts_with('$') {
                                            Style::Symbol
                                        } else {
                                            Style::File
                                        }
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Outputs",
                    no_children: outputs.is_empty()
                ) {
                    List {
                        #(outputs.iter().map(|output| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: output.as_str(),
                                        style: Style::File
                                    )
                                }
                            }
                        }))
                    }
                }
            }
        }
    })?;

    Ok(None)
}
