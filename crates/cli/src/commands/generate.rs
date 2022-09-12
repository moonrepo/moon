use crate::helpers::load_workspace;
use console::Term;
use dialoguer::{theme::Theme, Confirm, Input, MultiSelect, Select};
use moon_config::TemplateVariable;
use moon_error::MoonError;
use moon_generator::{FileState, Generator, GeneratorError, Template, TemplateContext};
use moon_logger::{color, warn};
use moon_terminal::create_theme;
use moon_utils::path;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

const LOG_TARGET: &str = "moon:generate";

#[derive(Debug)]
pub struct GenerateOptions {
    pub defaults: bool,
    pub dest: Option<String>,
    pub dry_run: bool,
    pub force: bool,
    pub template: bool,
    pub vars: Vec<String>,
}

fn format_var_name(name: &str) -> String {
    if name.starts_with("no-") {
        name.strip_prefix("no-").unwrap().to_owned()
    } else {
        name.to_owned()
    }
}

fn parse_var_args(vars: &[String]) -> HashMap<String, String> {
    let mut custom_vars = HashMap::new();

    let lexer = clap_lex::RawArgs::new(vars);
    let mut cursor = lexer.cursor();
    let mut previous_name: Option<String> = None;

    while let Some(arg) = lexer.next(&mut cursor) {
        // --name, --name=value
        if let Some((long, maybe_value)) = arg.to_long() {
            match long {
                Ok(name) => {
                    // If we found another long arg, but one previously exists,
                    // this must be a boolean value!
                    if let Some(name) = &previous_name {
                        custom_vars.insert(
                            format_var_name(name),
                            if name.starts_with("no-") {
                                "false".to_owned()
                            } else {
                                "true".to_owned()
                            },
                        );
                    }

                    // Value was explicitly defined with =
                    if let Some(value) = maybe_value {
                        previous_name = None;

                        custom_vars.insert(
                            format_var_name(name),
                            value.to_str().unwrap_or_default().to_owned(),
                        );

                        // No value defined, so persist the name till the next iteration
                    } else {
                        previous_name = Some(name.to_owned());
                    }
                }
                _ => {
                    warn!(
                        target: LOG_TARGET,
                        "Failed to parse argument --{}",
                        arg.display()
                    );
                }
            }

            // -n
        } else if arg.to_short().is_some() {
            warn!(
                target: LOG_TARGET,
                "Short arguments are not supported, found -{}",
                arg.display()
            );

            // value
        } else if let Some(name) = previous_name {
            previous_name = None;

            custom_vars.insert(
                format_var_name(&name),
                arg.to_value_os().to_str().unwrap_or_default().to_owned(),
            );
        }
    }

    custom_vars
}

fn gather_variables(
    template: &Template,
    theme: &dyn Theme,
    options: &GenerateOptions,
) -> Result<TemplateContext, GeneratorError> {
    let mut context = TemplateContext::new();
    let custom_vars = parse_var_args(&options.vars);
    let error_handler = |e| GeneratorError::Moon(MoonError::Io(e));

    for (name, config) in &template.config.variables {
        match config {
            TemplateVariable::Boolean(var) => {
                let default: bool = match custom_vars.get(name) {
                    Some(val) => val == "true",
                    None => var.default,
                };

                if options.defaults || var.prompt.is_none() {
                    context.insert(name, &default);
                } else {
                    let value = Confirm::with_theme(theme)
                        .default(default)
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .show_default(true)
                        .interact()
                        .map_err(error_handler)?;

                    context.insert(name, &value);
                }
            }
            TemplateVariable::Enum(var) => {
                let default = custom_vars.get(name).unwrap_or(&var.default);
                let default_index = var
                    .values
                    .iter()
                    .position(|i| i == default)
                    .unwrap_or_default();

                match (options.defaults, var.multiple.unwrap_or_default()) {
                    (true, true) => {
                        context.insert(name, &[&var.values[default_index]]);
                    }
                    (true, false) => {
                        context.insert(name, &var.values[default_index]);
                    }
                    (false, true) => {
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
                    }
                    (false, false) => {
                        let index = Select::with_theme(theme)
                            .with_prompt(&var.prompt)
                            .default(default_index)
                            .items(&var.values)
                            .interact()
                            .map_err(error_handler)?;

                        context.insert(name, &var.values[index]);
                    }
                };
            }
            TemplateVariable::Number(var) => {
                let required = var.required.unwrap_or_default();
                let default: i32 = match custom_vars.get(name) {
                    Some(val) => val.parse::<i32>().map_err(|e| {
                        GeneratorError::FailedToParseArgVar(name.to_owned(), e.to_string())
                    })?,
                    None => var.default,
                };

                if options.defaults || var.prompt.is_none() {
                    context.insert(name, &default);
                } else {
                    let value: i32 = Input::with_theme(theme)
                        .default(default)
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
                let default = custom_vars.get(name).unwrap_or(&var.default);

                if options.defaults || var.prompt.is_none() {
                    context.insert(name, &default);
                } else {
                    let value: String = Input::with_theme(theme)
                        .default(default.clone())
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .allow_empty(false)
                        .show_default(!default.is_empty())
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
    template.load_files(&dest, &context).await?;

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
