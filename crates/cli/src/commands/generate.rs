use crate::helpers::load_workspace;
use console::{style, Term};
use dialoguer::Input;
use moon_generator::Generator;
use moon_logger::color;
use moon_terminal::{create_theme, ExtendedTerm, Label};
use moon_utils::path;
use std::env;

#[derive(Debug)]
pub struct GenerateOptions {
    pub dest: Option<String>,
    pub template: bool,
}

pub async fn generate(
    name: &str,
    options: GenerateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let generator = Generator::create(&workspace.root, &workspace.config.generator)?;
    let cwd = env::current_dir()?;

    // This is a special case for creating a new template with the generator itself!
    if options.template {
        let template = generator.generate_template(name).await?;

        println!(
            "Created a new template {} at {}",
            color::id(template.name),
            color::path(template.root)
        );

        return Ok(());
    }

    // Create the template instance
    let term = Term::buffered_stdout();
    let template = generator.generate(name).await?;

    term.write_line(&color::id(format!(
        "{}",
        style(&template.config.title).bold()
    )))?;
    term.write_line(&template.config.description)?;
    term.write_line("")?;
    term.flush()?;

    // Determine the destination path
    let dest = match options.dest {
        Some(d) => d,
        None => Input::with_theme(&create_theme())
            .with_prompt("Where to generate code to?")
            .allow_empty(false)
            .interact_text()?,
    };
    let dest = path::normalize(cwd.join(dest));

    dbg!(template.get_template_files(&dest).await?);

    Ok(())
}
