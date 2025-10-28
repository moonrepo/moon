use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{Size, element};
use miette::IntoDiagnostic;
use moon_console::ui::{
    Container, Notice, Style, StyledText, Table, TableCol, TableHeader, TableRow, Variant,
};
use starbase::AppResult;
use starbase_utils::json;
use std::collections::BTreeMap;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TemplatesArgs {
    #[arg(long, help = "Filter templates based on this pattern")]
    filter: Option<String>,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn templates(session: MoonSession, args: TemplatesArgs) -> AppResult {
    let mut generator = session.build_code_generator();
    generator.load_templates().await?;

    if args.json {
        session
            .console
            .out
            .write_line(json::format(&generator.templates, true)?)?;

        return Ok(None);
    }

    if generator.templates.is_empty() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "No templates located. Configure them with the <property>generator.templates</property> setting.")
                }
            }
        })?;

        return Ok(None);
    }

    let mut templates = BTreeMap::from_iter(&generator.templates);

    if let Some(filter) = &args.filter {
        let pattern = regex::Regex::new(&format!("(?i){filter}")).into_diagnostic()?;

        templates.retain(|&id, _| pattern.is_match(id.as_str()));

        if templates.is_empty() {
            session.console.render(element! {
                Container {
                    Notice(variant: Variant::Caution) {
                        StyledText(content: "There are no templates that match the filter <shell>{filter}</shell>")
                    }
                }
            })?;

            return Ok(None);
        }
    }

    let id_width = templates
        .iter()
        .fold(0, |acc, (id, _)| acc.max(id.as_str().len()));

    let title_width = templates
        .iter()
        .fold(0, |acc, (_, template)| acc.max(template.config.title.len()));

    let vars_count = templates.iter().fold(0, |acc, (_, template)| {
        acc.max(template.config.variables.len())
    });

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Template", Size::Length((id_width + 5) as u32)),
                    TableHeader::new("Title", Size::Length((title_width + 5) as u32)),
                    TableHeader::new("Location", Size::Length(40)).hide_below(130),
                    TableHeader::new("Variables", Size::Length((vars_count * 2).clamp(10, 40) as u32)),
                    TableHeader::new("Description", Size::Auto).hide_below(100),
                ]
            ) {
                #(templates.into_iter().enumerate().map(|(i, (id, template))| {
                    let mut variables = template.config.variables.keys().collect::<Vec<_>>();
                    variables.sort();

                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: id.to_string(),
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                StyledText(
                                    content: &template.config.title,
                                    style: Style::Label
                                )
                            }
                            TableCol(col: 2) {
                                StyledText(
                                    content: match template.root.strip_prefix(&session.workspace_root) {
                                        Ok(root) => root.to_string_lossy(),
                                        Err(_) => template.root.to_string_lossy(),
                                    },
                                    style: Style::Path
                                )
                            }
                            TableCol(col: 3) {
                                StyledText(
                                    content: variables
                                        .iter()
                                        .map(|var| format!("<property>{var}</property>"))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                    style: Style::Muted
                                )
                            }
                            TableCol(col: 4) {
                                StyledText(
                                    content: &template.config.description,
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
