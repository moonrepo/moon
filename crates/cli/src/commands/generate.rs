use crate::helpers::load_workspace;
use console::{style, Term};
use dialoguer::Input;
use moon_generator::{FileState, Generator};
use moon_logger::color;
use moon_terminal::create_theme;
use moon_utils::path;
use std::env;
use std::path::PathBuf;

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
        let template = generator.create_template(name).await?;

        println!(
            "Created a new template {} at {}",
            color::id(template.name),
            color::path(template.root)
        );

        return Ok(());
    }

    // Create the template instance
    let template = generator.load_template(name).await?;
    let term = Term::buffered_stdout();

    term.write_line(&color::id(format!(
        "{}",
        style(&template.config.title).bold()
    )))?;
    term.write_line(&template.config.description)?;
    term.write_line("")?;
    term.flush()?;

    // Determine the destination path
    let relative_dest = match options.dest {
        Some(d) => d,
        None => Input::with_theme(&create_theme())
            .with_prompt("Where to generate code to?")
            .allow_empty(false)
            .interact_text()?,
    };
    let dest = path::normalize(cwd.join(&relative_dest));

    // Load template files and determine when to overwrite
    let files = template.get_template_files(&dest).await?;

    // Generate the files in the destination and print the results
    generator.generate(&files).await?;
    term.write_line("")?;

    for file in files {
        let file_state = file.state();

        term.write_line(&format!(
            "{} {} {}",
            match &file_state {
                FileState::Created => color::success("created"),
                FileState::Replaced => color::failure("replaced"),
                FileState::Skipped => color::invalid("skipped"),
            },
            match &file_state {
                FileState::Replaced => color::muted("->"),
                _ => color::muted("-->"),
            },
            color::muted_light(
                PathBuf::from(&relative_dest)
                    .join(file.path)
                    .to_string_lossy()
            )
        ))?;
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}
