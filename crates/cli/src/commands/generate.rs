use crate::helpers::load_workspace;
use console::Term;
use dialoguer::{theme::Theme, Confirm, Input, MultiSelect, Select};
use moon_config::TemplateVariable;
use moon_error::MoonError;
use moon_generator::{FileState, Generator, GeneratorError, Template, TemplateContext};
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

fn gather_variables(
    template: &Template,
    theme: &dyn Theme,
    options: &GenerateOptions,
) -> Result<TemplateContext, GeneratorError> {
    let mut context = TemplateContext::new();
    let error_handler = |e| GeneratorError::Moon(MoonError::Io(e));

    for (name, config) in &template.config.variables {
        match config {
            TemplateVariable::Boolean(var) => {
                if options.force || var.prompt.is_none() {
                    context.insert(name, &var.default);
                } else {
                    let value = Confirm::with_theme(theme)
                        .default(var.default)
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .show_default(true)
                        .interact()
                        .map_err(error_handler)?;

                    context.insert(name, &value);
                }
            }
            TemplateVariable::Enum(var) => {
                let default_index = var
                    .values
                    .iter()
                    .position(|i| i == &var.default)
                    .unwrap_or_default();

                if options.force {
                    context.insert(name, &var.values[default_index]);
                } else if var.multiple.unwrap_or_default() {
                    let indexes = MultiSelect::with_theme(theme)
                        .with_prompt(&var.prompt)
                        .items(&var.values)
                        .defaults(
                            &var.values
                                .iter()
                                .enumerate()
                                .map(|(i, _)| i == default_index)
                                .collect::<Vec<bool>>(),
                        )
                        .interact()
                        .map_err(error_handler)?;

                    context.insert(
                        name,
                        &indexes
                            .iter()
                            .map(|i| var.values[*i].clone())
                            .collect::<Vec<String>>(),
                    );
                } else {
                    let index = Select::with_theme(theme)
                        .with_prompt(&var.prompt)
                        .default(default_index)
                        .items(&var.values)
                        .interact()
                        .map_err(error_handler)?;

                    context.insert(name, &var.values[index]);
                }
            }
            TemplateVariable::Number(var) => {
                let required = var.required.unwrap_or_default();

                if options.force || var.prompt.is_none() {
                    context.insert(name, &var.default);
                } else {
                    let value: i32 = Input::with_theme(theme)
                        .default(var.default)
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .allow_empty(false)
                        .show_default(true)
                        .validate_with(|input: &i32| -> Result<(), &str> {
                            if required && *input == 0 {
                                Err("a non-zero value is required")
                            } else {
                                Ok(())
                            }
                        })
                        .interact_text()
                        .map_err(error_handler)?;

                    context.insert(name, &value);
                }
            }
            TemplateVariable::String(var) => {
                let required = var.required.unwrap_or_default();

                if options.force || var.prompt.is_none() {
                    context.insert(name, &var.default);
                } else {
                    let value: String = Input::with_theme(theme)
                        .default(var.default.clone())
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .allow_empty(false)
                        .show_default(!var.default.is_empty())
                        .validate_with(|input: &String| -> Result<(), &str> {
                            if required && input.is_empty() {
                                Err("a value is required")
                            } else {
                                Ok(())
                            }
                        })
                        .interact_text()
                        .map_err(error_handler)?;

                    context.insert(name, &value);
                }
            }
        }
    }

    Ok(context)
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
    let mut template = generator.load_template(name).await?;
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
    let relative_dest = match &options.dest {
        Some(d) => d.clone(),
        None => Input::with_theme(&theme)
            .with_prompt("Where to generate code to?")
            .allow_empty(false)
            .interact_text()?,
    };
    let dest = path::normalize(cwd.join(&relative_dest));

    // Gather variables and build context
    let context = gather_variables(&template, &theme, &options)?;

    // Load template files and determine when to overwrite
    template.load_files(&dest).await?;

    for file in &mut template.files {
        if file.dest_path.exists()
            && (options.force
                || Confirm::with_theme(&theme)
                    .with_prompt(format!(
                        "File {} already exists, overwrite?",
                        color::path(&file.dest_path)
                    ))
                    .interact()?)
        {
            file.overwrite = true;
        }
    }

    // Generate the files in the destination and print the results
    if !options.dry_run {
        generator.generate(&template, &context).await?;
    }

    term.write_line("")?;

    for file in template.files {
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
                    .join(&file.name)
                    .to_string_lossy()
            )
        ))?;
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}
