use crate::codegen::CodeGenerator;
use moon_console::Console;

pub async fn templates_command(
    mut generator: CodeGenerator<'_>,
    console: &Console,
) -> miette::Result<()> {
    let out = console.stdout();

    generator.load_templates().await?;

    out.print_header("Templates")?;

    for template in generator.templates.values() {
        out.print_entry_header(&template.config.title)?;
        out.write_line(&template.config.description)?;
        out.write_newline()?;
    }

    Ok(())
}
