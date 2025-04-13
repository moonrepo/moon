use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use miette::IntoDiagnostic;
use moon_codegen::CodeGenerator;
use moon_console::ui::{
    Container, Entry, List, ListItem, Notice, Section, Style, StyledText, Variant,
};
use starbase::AppResult;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TemplatesArgs {
    #[arg(long, help = "Filter the templates based on this pattern")]
    filter: Option<String>,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip_all)]
pub async fn templates(session: MoonSession, args: TemplatesArgs) -> AppResult {
    let mut generator = CodeGenerator::new(
        &session.workspace_root,
        &session.workspace_config.generator,
        Arc::clone(&session.moon_env),
    );

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

    session.console.render(element! {
        Container {
            #(templates.iter().map(|(id, template)| {
                let mut variables = template.config.variables.keys().collect::<Vec<_>>();
                variables.sort();

                element! {
                    Section(title: id.to_string()) {
                        Entry(
                            name: "Title",
                            value: element! {
                                StyledText(
                                    content: &template.config.title,
                                    style: Style::Label
                                )
                            }.into_any()
                        )
                        Entry(
                            name: "Description",
                            value: element! {
                                StyledText(
                                    content: &template.config.description,
                                    style: Style::MutedLight,
                                )
                            }.into_any()
                        )
                        Entry(
                            name: "Source location",
                            value: element! {
                                StyledText(
                                    content: template.root.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                        #(template.config.destination.as_ref().map(|dest| {
                            element! {
                                Entry(
                                    name: "Computed destination",
                                    value: element! {
                                        StyledText(
                                            content: dest,
                                            style: Style::File
                                        )
                                    }.into_any()
                                )
                            }
                        }))
                        #(if template.config.extends.is_empty() {
                            None
                        } else {
                            Some(element! {
                                Entry(
                                    name: "Extends from",
                                    value: element! {
                                        StyledText(
                                            content: template
                                                .config
                                                .extends
                                                .to_list()
                                                .iter()
                                                .map(|ef| format!("<id>{ef}</id>"))
                                                .collect::<Vec<_>>()
                                                .join(", "),
                                        )
                                    }.into_any()
                                )
                            })
                        })
                        #(if variables.is_empty() {
                            None
                        } else if variables.len() > 5 {
                            Some(element! {
                                Entry(
                                    name: "Supported variables",
                                ) {
                                    List {
                                        #(variables.into_iter().map(|var| {
                                            element! {
                                                ListItem {
                                                    StyledText(
                                                        content: var,
                                                        style: Style::Property
                                                    )
                                                }
                                            }
                                        }))
                                    }
                                }
                            })
                        } else {
                            Some(element! {
                                Entry(
                                    name: "Supported variables",
                                    value: element! {
                                        StyledText(
                                            content: variables
                                                .into_iter()
                                                .map(|var| format!("<property>{var}</property>"))
                                                .collect::<Vec<_>>()
                                                .join(", "),
                                        )
                                    }.into_any()
                                )
                            })
                        })
                    }
                }
            }))
        }
    })?;

    Ok(None)
}
