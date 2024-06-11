use crate::helpers::create_theme;
use crate::session::CliSession;
use dialoguer::{Confirm, Input, Select};
use miette::IntoDiagnostic;
use moon_codegen::{gather_variables, CodeGenerator, FileState};
use starbase::AppResult;
use starbase_styles::color;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, instrument};

pub use moon_codegen::GenerateArgs;

#[instrument(skip_all)]
pub async fn generate(session: CliSession, args: GenerateArgs) -> AppResult {
    let mut generator = CodeGenerator::new(
        &session.workspace_root,
        &session.workspace_config.generator,
        Arc::clone(&session.moon_env),
    );
    let console = session.console.stdout();
    let theme = create_theme();

    // This is a special case for creating a new template with the generator itself!
    if args.template {
        let template = generator.create_template(&args.name)?;

        console.write_line(format!(
            "Created a new template {} at {}",
            color::id(template.id),
            color::path(template.root)
        ))?;

        return Ok(());
    }

    if args.dry_run {
        debug!("Running in DRY MODE");
    }

    generator.load_templates().await?;

    // Create the template instance
    let mut template = generator.get_template(&args.name)?;

    console.write_newline()?;
    console.write_line(format!(
        "{} {}",
        &template.config.title,
        if args.dry_run {
            color::muted("(dry run)")
        } else {
            "".into()
        }
    ))?;
    console.write_line(&template.config.description)?;
    console.write_newline()?;
    console.flush()?;

    // Gather variables
    let mut context = gather_variables(&args, &template, &session.console)?;

    // Determine the destination path
    let relative_dest = PathBuf::from(match &args.dest {
        Some(dest) => dest.to_owned(),
        None => {
            if let Some(dest) = &template.config.destination {
                debug!(dest, "Default destination path provided by template config");

                dest.to_owned()
            } else {
                debug!("Destination path not provided, prompting the user");

                Input::with_theme(&theme)
                    .with_prompt("Where to generate code to?")
                    .allow_empty(false)
                    .interact_text()
                    .into_diagnostic()?
            }
        }
    });
    let relative_dest = template.interpolate_path(&relative_dest, &context)?;
    let dest = relative_dest.to_logical_path(&session.working_dir);

    debug!(dest = ?dest, "Destination path set");

    // Inject built-in context variables
    context.insert("dest_dir", &dest);
    context.insert("dest_rel_dir", &relative_dest);
    context.insert("working_dir", &session.working_dir);
    context.insert("workspace_root", &session.workspace_root);

    // Load template files and determine when to overwrite
    template.load_files(&dest, &context)?;

    for file in template.files.values_mut() {
        if file.is_skipped() {
            file.state = FileState::Skip;
            continue;
        }

        if file.dest_path.exists() {
            if args.force || file.is_forced() {
                file.state = FileState::Replace;
                continue;
            }

            // Merge files when applicable
            if file.is_mergeable().is_some() {
                let operations = [
                    "Keep existing file",
                    "Merge new file into existing file",
                    "Replace existing with new file",
                ];

                let index = Select::with_theme(&theme)
                    .with_prompt(format!(
                        "File {} already exists, what to do?",
                        color::path(&file.dest_path)
                    ))
                    .default(2)
                    .items(&operations)
                    .interact()
                    .into_diagnostic()?;

                file.state = match index {
                    0 => FileState::Skip,
                    1 => FileState::Merge,
                    _ => FileState::Replace,
                };

                continue;
            }

            // Confirm whether to replace file
            if Confirm::with_theme(&theme)
                .with_prompt(format!(
                    "File {} already exists, overwrite?",
                    color::path(&file.dest_path)
                ))
                .interact()
                .into_diagnostic()?
            {
                file.state = FileState::Replace;
            }
        }
    }

    // Generate the files in the destination and print the results
    if !args.dry_run {
        generator.generate(&template)?;
    }

    console.write_newline()?;

    for file in template.files.values() {
        console.write_line(format!(
            "{} {} {}",
            match &file.state {
                FileState::Create => color::success("created"),
                FileState::Merge => color::success("merged"),
                FileState::Replace => color::failure("replaced"),
                FileState::Skip => color::invalid("skipped"),
            },
            match &file.state {
                FileState::Merge => color::muted("--->"),
                FileState::Replace => color::muted("->"),
                _ => color::muted("-->"),
            },
            color::muted_light(
                file.dest_path
                    .strip_prefix(&session.working_dir)
                    .unwrap_or(&file.dest_path)
                    .to_string_lossy()
            )
        ))?;
    }

    console.write_newline()?;
    console.flush()?;

    Ok(())
}
