use crate::helpers::load_workspace;
use console::Term;
use dialoguer::{Confirm, Input};
use moon_generator::{FileState, Generator};
use moon_logger::color;
use moon_terminal::create_theme;
use moon_utils::path;
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
pub struct GenerateOptions {
    pub dest: Option<String>,
    pub dry_run: bool,
    pub force: bool,
    pub template: bool,
}

pub async fn generate(
    name: &str,
    options: GenerateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;
    let generator = Generator::create(&workspace.root, &workspace.config.generator)?;
    let theme = create_theme();
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

    term.write_line("")?;
    term.write_line(&format!(
        "{} {}",
        color::style(&template.config.title).bold(),
        if options.dry_run {
            color::muted("(dry run)")
        } else {
            "".into()
        }
    ))?;
    term.write_line(&template.config.description)?;
    term.write_line("")?;
    term.flush()?;

    // Determine the destination path
    let relative_dest = match options.dest {
        Some(d) => d,
        None => Input::with_theme(&theme)
            .with_prompt("Where to generate code to?")
            .allow_empty(false)
            .interact_text()?,
    };
    let dest = path::normalize(cwd.join(&relative_dest));

    // Load template files and determine when to overwrite
    let mut files = template.get_template_files(&dest).await?;

    for file in &mut files {
        if file.dest_path.exists() {
            if options.force
                || Confirm::with_theme(&theme)
                    .with_prompt(format!(
                        "File {} already exists, overwrite?",
                        color::path(&file.dest_path)
                    ))
                    .interact()?
            {
                file.overwrite = true;
            }
        }
    }

    // Generate the files in the destination and print the results
    if !options.dry_run {
        generator.generate(&files).await?;
    }

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
