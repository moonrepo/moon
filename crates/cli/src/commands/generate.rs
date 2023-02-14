use crate::helpers::AnyError;
use console::Term;
use dialoguer::{theme::Theme, Confirm, Input, MultiSelect, Select};
use moon::load_workspace;
use moon_config::{TemplateVariable, TemplateVariableEnumValue};
use moon_error::MoonError;
use moon_generator::{FileState, Generator, GeneratorError, Template, TemplateContext};
use moon_logger::{color, debug, map_list, trace, warn};
use moon_terminal::create_theme;
use moon_utils::path;
use rustc_hash::FxHashMap;
use std::env;
use std::fmt::Display;
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

fn log_var<T: Display>(name: &str, value: &T, comment: Option<&str>) {
    trace!(
        target: LOG_TARGET,
        "Setting variable {} to \"{}\" {}",
        color::id(name),
        value,
        comment.unwrap_or_default(),
    );
}

fn parse_var_args(vars: &[String]) -> FxHashMap<String, String> {
    let mut custom_vars = FxHashMap::default();

    if vars.is_empty() {
        return custom_vars;
    }

    debug!(
        target: LOG_TARGET,
        "Inheriting variable values from provided command line arguments"
    );

    let lexer = clap_lex::RawArgs::new(vars);
    let mut cursor = lexer.cursor();
    let mut previous_name: Option<String> = None;

    let mut set_var = |name: &str, value: String| {
        let name = if let Some(stripped_name) = name.strip_prefix("no-") {
            stripped_name
        } else {
            name
        };
        let comment = color::muted_light(format!("(from --{name})"));

        log_var(name, &value, Some(&comment));

        custom_vars.insert(name.to_owned(), value);
    };

    while let Some(arg) = lexer.next(&mut cursor) {
        // --name, --name=value
        if let Some((long, maybe_value)) = arg.to_long() {
            match long {
                Ok(name) => {
                    // If we found another long arg, but one previously exists,
                    // this must be a boolean value!
                    if let Some(name) = &previous_name {
                        set_var(
                            name,
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

                        set_var(name, value.to_str().unwrap_or_default().to_owned());

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

            set_var(
                &name,
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
    let default_comment = color::muted_light("(defaults)");

    debug!(
        target: LOG_TARGET,
        "Declaring variable values from defaults and user prompts"
    );

    for (name, config) in &template.config.variables {
        match config {
            TemplateVariable::Boolean(var) => {
                let default: bool = match custom_vars.get(name) {
                    Some(val) => val == "true",
                    None => var.default,
                };

                if options.defaults || var.prompt.is_none() {
                    log_var(name, &default, Some(&default_comment));

                    context.insert(name, &default);
                } else {
                    let value = Confirm::with_theme(theme)
                        .default(default)
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .show_default(true)
                        .interact()
                        .map_err(error_handler)?;

                    log_var(name, &value, None);

                    context.insert(name, &value);
                }
            }
            TemplateVariable::Enum(var) => {
                let values = var
                    .values
                    .iter()
                    .map(|e| match e {
                        TemplateVariableEnumValue::String(value) => value,
                        TemplateVariableEnumValue::Object { value, .. } => value,
                    })
                    .collect::<Vec<_>>();
                let labels = var
                    .values
                    .iter()
                    .map(|e| match e {
                        TemplateVariableEnumValue::String(value) => value,
                        TemplateVariableEnumValue::Object { label, .. } => label,
                    })
                    .collect::<Vec<_>>();

                let default = custom_vars.get(name).unwrap_or(&var.default);
                let default_index = values
                    .iter()
                    .position(|i| *i == default)
                    .unwrap_or_default();

                if options.defaults {
                    log_var(name, &values[default_index], Some(&default_comment));
                }

                match (options.defaults, var.multiple.unwrap_or_default()) {
                    (true, true) => {
                        context.insert(name, &[&values[default_index]]);
                    }
                    (true, false) => {
                        context.insert(name, &values[default_index]);
                    }
                    (false, true) => {
                        let indexes = MultiSelect::with_theme(theme)
                            .with_prompt(&var.prompt)
                            .items(&labels)
                            .defaults(
                                &values
                                    .iter()
                                    .enumerate()
                                    .map(|(i, _)| i == default_index)
                                    .collect::<Vec<bool>>(),
                            )
                            .interact()
                            .map_err(error_handler)?;
                        let value = indexes
                            .iter()
                            .map(|i| values[*i].clone())
                            .collect::<Vec<String>>();

                        log_var(name, &map_list(&value, |f| f.to_string()), None);

                        context.insert(name, &value);
                    }
                    (false, false) => {
                        let index = Select::with_theme(theme)
                            .with_prompt(&var.prompt)
                            .default(default_index)
                            .items(&labels)
                            .interact()
                            .map_err(error_handler)?;

                        log_var(name, &values[index], None);

                        context.insert(name, &values[index]);
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
                    log_var(name, &default, Some(&default_comment));

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

                    log_var(name, &value, None);

                    context.insert(name, &value);
                }
            }
            TemplateVariable::String(var) => {
                let required = var.required.unwrap_or_default();
                let default = custom_vars.get(name).unwrap_or(&var.default);

                if options.defaults || var.prompt.is_none() {
                    log_var(name, &default, Some(&default_comment));

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

                    log_var(name, &value, None);

                    context.insert(name, &value);
                }
            }
        }
    }

    Ok(context)
}

pub async fn generate(name: String, options: GenerateOptions) -> Result<(), AnyError> {
    let workspace = load_workspace().await?;
    let generator = Generator::load(&workspace.root, &workspace.config.generator)?;
    let theme = create_theme();
    let cwd = env::current_dir()?;

    // This is a special case for creating a new template with the generator itself!
    if options.template {
        let template = generator.create_template(&name)?;

        println!(
            "Created a new template {} at {}",
            color::id(template.name),
            color::path(template.root)
        );

        return Ok(());
    }

    if options.dry_run {
        debug!(target: LOG_TARGET, "Running in DRY MODE");
    }

    // Create the template instance
    let mut template = generator.load_template(&name)?;
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
        None => {
            trace!(
                target: LOG_TARGET,
                "Destination path not provided, prompting the user"
            );

            Input::with_theme(&theme)
                .with_prompt("Where to generate code to?")
                .allow_empty(false)
                .interact_text()?
        }
    };
    let dest = path::normalize(cwd.join(&relative_dest));

    debug!(
        target: LOG_TARGET,
        "Destination path set to {}",
        color::path(&dest)
    );

    // Gather variables and build context
    let mut context = gather_variables(&template, &theme, &options)?;
    context.insert("dest_dir", &dest);
    context.insert("dest_rel_dir", &relative_dest);
    context.insert("working_dir", &cwd);
    context.insert("workspace_root", &workspace.root);

    // Load template files and determine when to overwrite
    template.load_files(&dest, &context)?;

    for file in &mut template.files {
        if file.is_skipped() {
            file.state = FileState::Skip;
            continue;
        }

        if file.dest_path.exists() {
            if options.force || file.is_forced() {
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
                    .interact()?;

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
                .interact()?
            {
                file.state = FileState::Replace;
            }
        }
    }

    // Generate the files in the destination and print the results
    if !options.dry_run {
        generator.generate(&template)?;
    }

    term.write_line("")?;

    for file in template.files {
        term.write_line(&format!(
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
