use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::{View, element};
use moon_codegen::{Template, TemplateContext};
use moon_common::{Id, is_test_env};
use moon_config::TemplateVariable;
use moon_console::ui::{Container, Entry, List, ListItem, Section, Stack, Style, StyledText};
use starbase::AppResult;
use starbase_utils::json;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct TemplateArgs {
    #[arg(help = "Template ID to inspect")]
    id: Id,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[instrument(skip(session))]
pub async fn template(session: MoonSession, args: TemplateArgs) -> AppResult {
    let mut generator = session.build_code_generator();
    generator.load_templates().await?;

    let mut template = generator.get_template(&args.id)?;
    let context = create_default_context(&session, &template);
    template.load_files(&session.workspace_root, &context)?;

    if args.json {
        session
            .console
            .out
            .write_line(json::format(&template, true)?)?;

        return Ok(None);
    }

    let show_in_prod = !is_test_env();

    session.console.render(element! {
        Container {
            Section(title: &template.config.title) {
                StyledText(
                    content: &template.config.description,
                )
            }

            Section(title: "About") {
                Entry(
                    name: "Template",
                    value: element! {
                        StyledText(
                            content: template.id.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                #(show_in_prod.then(|| {
                    element! {
                        Entry(
                            name: "Location",
                            value: element! {
                                StyledText(
                                    content: template.root.to_string_lossy(),
                                    style: Style::Path
                                )
                            }.into_any()
                        )
                    }
                }))
                #(template.config.destination.as_ref().map(|dest| {
                    element! {
                        Entry(
                            name: "Destination",
                            value: element! {
                                StyledText(
                                    content: dest,
                                    style: Style::File
                                )
                            }.into_any()
                        )
                    }
                }))
                Entry(
                    name: "Extends",
                    no_children: template.config.extends.is_empty()
                ) {
                    List {
                        #(template.config.extends.to_list().iter().map(|dep| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: dep.to_string(),
                                        style: Style::Id
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Assets",
                    no_children: template.assets.is_empty()
                ) {
                    List {
                        #(template.assets.values().map(|asset| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: asset.name.as_str(),
                                        style: Style::File
                                    )
                                }
                            }
                        }))
                    }
                }
                Entry(
                    name: "Files",
                    no_children: template.files.is_empty()
                ) {
                    List {
                        #(template.files.values().map(|file| {
                            element! {
                                ListItem {
                                    StyledText(
                                        content: file.name.as_str(),
                                        style: Style::File
                                    )
                                }
                            }
                        }))
                    }
                }
            }

            #((!template.config.variables.is_empty()).then(|| {
                element! {
                    Section(title: "Variables") {
                        Stack(gap: 1) {
                            #(template.config.variables.iter().map(|(name, var)| {
                                let mut flags = vec![];

                                if var.is_internal() {
                                    flags.push("internal");
                                }

                                if var.is_multiple() {
                                    flags.push("multiple");
                                }

                                if var.is_required() {
                                    flags.push("required");
                                }

                                element! {
                                    Stack {
                                        View {
                                            StyledText(
                                                content: format!(
                                                    "<property>{}</property><muted>:</muted> {} {}",
                                                    name,
                                                    get_variable_type(var),
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
                                        #(var.get_prompt().map(|prompt| {
                                            element! {
                                                View {
                                                    StyledText(
                                                        content: prompt,
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
                }
            }))
        }
    })?;

    Ok(None)
}

fn create_default_context(session: &MoonSession, template: &Template) -> TemplateContext {
    let mut context = TemplateContext::default();
    context.insert("working_dir", &session.working_dir);
    context.insert("workspace_root", &session.workspace_root);
    context.insert("dest_dir", &session.working_dir);
    context.insert("dest_rel_dir", &session.working_dir);

    for (name, var) in &template.config.variables {
        match var {
            TemplateVariable::Array(cfg) => context.insert(name, &cfg.default),
            TemplateVariable::Boolean(cfg) => context.insert(name, &cfg.default),
            TemplateVariable::Enum(cfg) => context.insert(name, &cfg.default),
            TemplateVariable::Number(cfg) => context.insert(name, &cfg.default),
            TemplateVariable::Object(cfg) => context.insert(name, &cfg.default),
            TemplateVariable::String(cfg) => context.insert(name, &cfg.default),
        };
    }

    context
}

fn get_variable_type(var: &TemplateVariable) -> String {
    // Matches the schematic style
    match var {
        TemplateVariable::Array(_) => "[bool | number | string]",
        TemplateVariable::Boolean(_) => "bool",
        TemplateVariable::Enum(_) => "string | [string]",
        TemplateVariable::Number(_) => "number",
        TemplateVariable::Object(_) => "{string: bool | number | string}",
        TemplateVariable::String(_) => "string",
    }
    .into()
}
