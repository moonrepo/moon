use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{Size, element};
use moon_common::Id;
use moon_console::ui::{Container, Style, StyledText, Table, TableCol, TableHeader, TableRow};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TasksArgs {
    #[arg(help = "Filter tasks to a specific project")]
    project: Option<Id>,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn tasks(session: MoonSession, args: TasksArgs) -> AppResult {
    let mut tasks = session.get_workspace_graph().await?.get_tasks()?;

    if let Some(project_id) = &args.project {
        tasks = tasks
            .into_iter()
            .filter(|task| {
                task.target
                    .get_project_id()
                    .is_ok_and(|id| project_id == id)
            })
            .collect();
    }

    tasks.sort_by(|a, d| a.target.cmp(&d.target));

    if args.json {
        session
            .console
            .out
            .write_line(json::format(&tasks, true)?)?;

        return Ok(None);
    }

    let id_width = tasks
        .iter()
        .fold(0, |acc, task| acc.max(task.target.as_str().len()));
    let command_width = tasks
        .iter()
        .fold(0, |acc, task| acc.max(task.command.len()));

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Task", Size::Length((id_width + 5).max(10) as u32)),
                    TableHeader::new("Command", Size::Length((command_width + 5) as u32)),
                    TableHeader::new("Type", Size::Length(10)).hide_below(130),
                    TableHeader::new("Preset", Size::Length(10)).hide_below(160),
                    TableHeader::new("Toolchains", Size::Length(40)),
                    TableHeader::new("Description", Size::Auto).hide_below(100),
                ]
            ) {
                #(tasks.into_iter().enumerate().map(|(i, task)| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: task.target.to_string(),
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                StyledText(
                                    content: if task.script.is_some() {
                                        "(script)"
                                    } else {
                                        &task.command
                                    },
                                    style: Style::Shell
                                )
                            }
                            TableCol(col: 2) {
                                StyledText(
                                    content: task.type_of.to_string(),
                                )
                            }
                            TableCol(col: 3) {
                                #(task.preset.as_ref().map(|preset| {
                                    element! {
                                        StyledText(
                                            content: preset.to_string(),
                                        )
                                    }
                                }))
                            }
                            TableCol(col: 4) {
                                StyledText(
                                    content: task.toolchains.join(", "),
                                )
                            }
                            TableCol(col: 5) {
                                StyledText(
                                    content: task.description.as_deref().unwrap_or(""),
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
