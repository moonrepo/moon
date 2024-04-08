use crate::codegen::CodeGenerator;
use moon_common::color;
use moon_console::Console;

pub async fn templates_command(
    mut generator: CodeGenerator<'_>,
    console: &Console,
) -> miette::Result<()> {
    generator.load_templates().await?;

    let out = console.stdout();

    for template in generator.templates.values() {
        out.print_entry_header(&template.id)?;

        out.write_line(format!(
            "{} {} {}",
            color::label(&template.config.title),
            color::muted("-"),
            template.config.description
        ))?;

        out.print_entry("Source location", color::path(&template.root))?;

        if let Some(destination) = &template.config.destination {
            out.print_entry("Default destination", color::file(destination))?;
        }

        if !template.config.extends.is_empty() {
            out.print_entry(
                "Extends from",
                template
                    .config
                    .extends
                    .iter()
                    .map(|ext| color::id(ext))
                    .collect::<Vec<_>>()
                    .join(&color::muted(", ")),
            )?;
        }

        if !template.config.variables.is_empty() {
            out.print_entry(
                "Supported variables",
                template
                    .config
                    .variables
                    .keys()
                    .map(|ext| color::property(ext))
                    .collect::<Vec<_>>()
                    .join(&color::muted(", ")),
            )?;
        }
    }

    out.write_newline()?;
    out.flush()?;

    Ok(())
}
