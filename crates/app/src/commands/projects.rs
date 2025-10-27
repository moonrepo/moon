use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{Size, element};
use moon_console::ui::{Container, Style, StyledText, Table, TableCol, TableHeader, TableRow};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ProjectsArgs {}

#[instrument(skip(session))]
pub async fn projects(session: MoonSession) -> AppResult {
    let mut projects = session.get_workspace_graph().await?.get_projects()?;

    projects.sort_by(|a, d| a.id.cmp(&d.id));

    let id_width = projects
        .iter()
        .fold(0, |acc, project| acc.max(project.id.as_str().len()));
    let source_width = projects
        .iter()
        .fold(0, |acc, project| acc.max(project.source.as_str().len()));

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Project", Size::Length((id_width + 5).max(10) as u32)),
                    TableHeader::new("Source", Size::Length((source_width + 5) as u32)),
                    TableHeader::new("Stack", Size::Length(16)).hide_below(160),
                    TableHeader::new("Layer", Size::Length(16)).hide_below(130),
                    TableHeader::new("Toolchains", Size::Length(40)),
                    TableHeader::new("Description", Size::Auto).hide_below(100),
                ]
            ) {
                #(projects.into_iter().enumerate().map(|(i, project)| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: project.id.to_string(),
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                StyledText(
                                    content: project.source.to_string(),
                                    style: Style::File
                                )
                            }
                            TableCol(col: 2) {
                                StyledText(
                                    content: project.stack.to_string(),
                                )
                            }
                            TableCol(col: 3) {
                                StyledText(
                                    content: project.layer.to_string(),
                                )
                            }
                            TableCol(col: 4) {
                                StyledText(
                                    content: project.toolchains.join(", "),
                                )
                            }
                            TableCol(col: 5) {
                                StyledText(
                                    content: project
                                        .config
                                        .project
                                        .as_ref()
                                        .and_then(|cfg| cfg.description.as_deref())
                                        .unwrap_or(""),
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
