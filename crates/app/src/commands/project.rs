use crate::session::MoonSession;
use clap::Args;
use convert_case::{Case, Casing};
use iocraft::prelude::{View, element};
use moon_common::{Id, is_test_env};
use moon_console::ui::{
    Container, Entry, List, ListItem, Map, MapItem, Section, Style, StyledText,
};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ProjectArgs {
    #[arg(help = "ID of project to display")]
    id: Id,

    #[arg(long, help = "Print in JSON format")]
    json: bool,

    #[arg(long, help = "Do not include tasks in output")]
    no_tasks: bool,
}

#[instrument(skip_all)]
pub async fn project(session: MoonSession, args: ProjectArgs) -> AppResult {
    let workspace_graph = session.get_workspace_graph().await?;
    let project = workspace_graph.get_project_with_tasks(&args.id)?;
    let config = &project.config;
    let console = &session.console;

    if args.json {
        console.out.write_line(json::format(&project, true)?)?;

        return Ok(None);
    }

    let show_in_prod = !is_test_env();
    let toolchains = project.get_enabled_toolchains();

    session.console.render(element! {
        Container {
            #(config.project.as_ref().map(|meta| {
                element! {
                    Section(title: "Metadata") {
                        View(margin_bottom: 1) {
                            StyledText(
                                content: &meta.description,
                            )
                        }
                        #(meta.name.as_ref().map(|name| {
                            element! {
                                Entry(
                                    name: "Name",
                                    content: name.to_string(),
                                )
                            }
                        }))
                        #(meta.channel.as_ref().map(|channel| {
                            element! {
                                Entry(
                                    name: "Channel",
                                    content: channel.to_string(),
                                )
                            }
                        }))
                        #(meta.owner.as_ref().map(|owner| {
                            element! {
                                Entry(
                                    name: "Owner",
                                    content: owner.to_string(),
                                )
                            }
                        }))
                        #(if meta.maintainers.is_empty() {
                            None
                        } else {
                            Some(element! {
                                Entry(
                                    name: "Maintainers",
                                    content: meta.maintainers.join(", "),
                                )
                            })
                        })
                        #(meta.metadata.iter().map(|(key, value)| {
                            element! {
                                 Entry(
                                    name: key.to_case(Case::Title),
                                    content: value.to_string(),
                                )
                            }
                        }))
                    }
                }
            }))

            Section(title: "About") {
                Entry(
                    name: "Project",
                    value: element! {
                        StyledText(
                            content: project.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                #(project.alias.as_ref().map(|alias| {
                    element! {
                        Entry(
                            name: "Alias",
                            value: element! {
                                StyledText(
                                    content: alias,
                                    style: Style::Label
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: "Source",
                    value: element! {
                        StyledText(
                            content: project.source.to_string(),
                            style: Style::File
                        )
                    }.into_any()
                )
                #(show_in_prod.then(|| {
                    element! {
                        Entry(
                            name: "Root",
                            value: element! {
                                StyledText(
                                    content: project.root.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: if toolchains.len() == 1 {
                        "Toolchain"
                    } else {
                        "Toolchains"
                    },
                    content: toolchains.into_iter().cloned().collect::<Vec<_>>().join(", "),
                )
                Entry(
                    name: "Language",
                    content: project.language.to_string(),
                )
                Entry(
                    name: "Stack",
                    content: project.stack.to_string(),
                )
                Entry(
                    name: "Layer",
                    content: project.layer.to_string(),
                )
                 Entry(
                    name: "Depends on",
                    no_children: project.dependencies.is_empty()
                ) {
                    List {
                        #(project.dependencies.iter().map(|dep| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: format!(
                                            "{} <mutedlight>({})</mutedlight>",
                                            dep.id,
                                            if let Some(via) = &dep.via {
                                                format!("{} via {via}", dep.scope)
                                            } else {
                                                dep.scope.to_string()
                                            },
                                        ),
                                        style: Style::Id
                                    )
                                }
                            }
                        }))
                    }
                }
            }

            Section(title: "Configuration") {
                #(project.inherited.as_ref().map(|inherited| {
                    element! {
                        Entry(
                            name: "Inherits from",
                            no_children: inherited.layers.is_empty()
                        ) {
                            List {
                                #(inherited.layers.keys().map(|layer| {
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
                    name: "Tags",
                    no_children: config.tags.is_empty()
                ) {
                    List {
                        #(config.tags.iter().map(|tag| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: tag.as_str(),
                                        style: Style::Id
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Environment variables",
                    no_children: config.env.is_empty()
                ) {
                    Map {
                        #(config.env.iter().map(|(key, value)| {
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
            }

            #((!project.file_groups.is_empty()).then(|| {
                element! {
                    Section(title: "File groups") {
                        #(project.file_groups.iter().map(|(id, group)| {
                            let mut files = vec![];
                            files.extend(&group.files);
                            files.extend(&group.globs);
                            files.sort();

                            element! {
                                Entry(name: id.as_str()) {
                                    List {
                                        #(files.into_iter().map(|file| {
                                            element! {
                                                ListItem {
                                                    StyledText(
                                                        content: file.as_str(),
                                                        style: Style::File
                                                    )
                                                }
                                            }
                                        }))
                                    }
                                }
                            }
                        }))
                    }
                }
            }))

            #((!project.tasks.is_empty() && !args.no_tasks).then(|| {
                element! {
                    Section(title: "Tasks") {
                        #(project.tasks.values().map(|task| {
                            element! {
                                Entry(name: task.id.as_str()) {
                                    View {
                                        StyledText(
                                            content: "› ",
                                            style: Style::Muted
                                        )
                                        StyledText(
                                            content: task.get_command_line(),
                                            style: Style::Shell
                                        )
                                    }
                                }
                            }
                        }))
                    }
                }
            }))
        }
    })?;

    // if !project.file_groups.is_empty() {
    //     console.print_entry_header("File groups")?;

    //     for group_name in project.file_groups.keys() {
    //         let mut files = vec![];
    //         let group = project.file_groups.get(group_name).unwrap();

    //         for file in &group.files {
    //             files.push(color::file(file));
    //         }

    //         for file in &group.globs {
    //             files.push(color::file(file));
    //         }

    //         console.print_entry_list(group_name, files)?;
    //     }
    // }

    // console.write_newline()?;
    // console.flush()?;

    Ok(None)
}
