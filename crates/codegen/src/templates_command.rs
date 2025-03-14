use crate::codegen::CodeGenerator;
use clap::Args;
use miette::IntoDiagnostic;
use moon_common::color;
use moon_console::MoonConsole;
use std::collections::BTreeMap;

#[derive(Args, Clone, Debug)]
pub struct TemplatesArgs {
    #[arg(long, help = "Filter the templates based on this pattern")]
    pub filter: Option<String>,
}

pub async fn templates_command(
    mut generator: CodeGenerator<'_>,
    console: &MoonConsole,
    args: &TemplatesArgs,
) -> miette::Result<Option<u8>> {
    generator.load_templates().await?;

    let mut templates = BTreeMap::from_iter(&generator.templates);

    if templates.is_empty() {
        console
            .err
            .write_line("There are no configured templates")?;

        return Ok(Some(1));
    }

    if let Some(filter) = &args.filter {
        let pattern = regex::Regex::new(&format!("(?i){filter}")).into_diagnostic()?;

        templates.retain(|&id, _| pattern.is_match(id.as_str()));

        if templates.is_empty() {
            console.err.write_line(format!(
                "There are no templates that match the filter {}",
                color::shell(filter)
            ))?;

            return Ok(Some(1));
        }
    }

    for (_, template) in templates {
        // TODO
        // console.print_entry_header(&template.id)?;

        console.out.write_line(format!(
            "{} {} {}",
            color::label(&template.config.title),
            color::muted("-"),
            template.config.description
        ))?;

        console.print_entry("Source location", color::path(&template.root))?;

        if let Some(destination) = &template.config.destination {
            console.print_entry("Default destination", color::file(destination))?;
        }

        if !template.config.extends.is_empty() {
            console.print_entry(
                "Extends from",
                template
                    .config
                    .extends
                    .to_list()
                    .iter()
                    .map(color::id)
                    .collect::<Vec<_>>()
                    .join(&color::muted(", ")),
            )?;
        }

        if !template.config.variables.is_empty() {
            console.print_entry(
                "Supported variables",
                template
                    .config
                    .variables
                    .keys()
                    .map(color::property)
                    .collect::<Vec<_>>()
                    .join(&color::muted(", ")),
            )?;
        }
    }

    console.out.write_newline()?;
    console.out.flush()?;

    Ok(None)
}
