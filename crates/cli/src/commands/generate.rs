use clap::Args;
use console::Term;
use dialoguer::{theme::Theme, Confirm, Input, MultiSelect, Select};
use miette::IntoDiagnostic;
use moon::load_workspace;
use moon_codegen::{CodeGenerator, CodegenError, FileState, Template, TemplateContext};
use moon_common::path::RelativePathBuf;
use moon_config::{TemplateVariable, TemplateVariableEnumValue};
use moon_logger::map_list;
use moon_terminal::{create_theme, ExtendedTerm};
use rustc_hash::FxHashMap;
use starbase::AppResult;
use starbase_styles::color;
use std::env;
use std::fmt::Display;
use tracing::{debug, warn};

#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    #[arg(help = "Name of template to generate")]
    name: String,

    #[arg(help = "Destination path, relative from the current working directory")]
    dest: Option<String>,

    #[arg(
        long,
        help = "Use the default value of all variables instead of prompting"
    )]
    defaults: bool,

    #[arg(
        long = "dryRun",
        help = "Run entire generator process without writing files"
    )]
    dry_run: bool,

    #[arg(long, help = "Force overwrite any existing files at the destination")]
    force: bool,

    #[arg(long, help = "Create a new template")]
    template: bool,

    // Variable args (after --)
    #[arg(last = true, help = "Arguments to define as variable values")]
    vars: Vec<String>,
}

fn log_var<T: Display>(name: &str, value: &T, comment: Option<&str>) {
    debug!(name, value = %value, comment, "Setting variable");
}

fn parse_var_args(vars: &[String]) -> FxHashMap<String, String> {
    let mut custom_vars = FxHashMap::default();

    if vars.is_empty() {
        return custom_vars;
    }

    debug!("Inheriting variable values from command line arguments");

    let lexer = clap_lex::RawArgs::new(vars);
    let mut cursor = lexer.cursor();
    let mut previous_name: Option<String> = None;

    let mut set_var = |name: &str, value: String| {
        let name = if let Some(stripped_name) = name.strip_prefix("no-") {
            stripped_name
        } else {
            name
        };
        let comment = format!("(from --{name})");

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
                    warn!("Failed to parse argument --{}", arg.display());
                }
            }

            // -n
        } else if arg.to_short().is_some() {
            warn!(
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
    args: &GenerateArgs,
) -> AppResult<TemplateContext> {
    let mut context = TemplateContext::new();
    let custom_vars = parse_var_args(&args.vars);
    let default_comment = "(defaults)";

    debug!("Declaring variable values from defaults and user prompts");

    for (name, config) in &template.config.variables {
        match config {
            TemplateVariable::Boolean(var) => {
                let default: bool = match custom_vars.get(name) {
                    Some(val) => val == "true",
                    None => var.default,
                };

                if args.defaults || var.prompt.is_none() {
                    log_var(name, &default, Some(default_comment));

                    context.insert(name, &default);
                } else {
                    let value = Confirm::with_theme(theme)
                        .default(default)
                        .with_prompt(var.prompt.as_ref().unwrap())
                        .show_default(true)
                        .interact()
                        .into_diagnostic()?;

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
                        TemplateVariableEnumValue::Object(cfg) => &cfg.value,
                    })
                    .collect::<Vec<_>>();
                let labels = var
                    .values
                    .iter()
                    .map(|e| match e {
                        TemplateVariableEnumValue::String(value) => value,
                        TemplateVariableEnumValue::Object(cfg) => &cfg.label,
                    })
                    .collect::<Vec<_>>();

                let default = custom_vars.get(name).unwrap_or(&var.default);
                let default_index = values
                    .iter()
                    .position(|i| *i == default)
                    .unwrap_or_default();

                if args.defaults {
                    log_var(name, &values[default_index], Some(default_comment));
                }

                match (args.defaults, var.multiple.unwrap_or_default()) {
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
                            .into_diagnostic()?;
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
                            .into_diagnostic()?;

                        log_var(name, &values[index], None);

                        context.insert(name, &values[index]);
                    }
                };
            }
            TemplateVariable::Number(var) => {
                let required = var.required.unwrap_or_default();
                let default: i32 = match custom_vars.get(name) {
                    Some(val) => val
                        .parse::<i32>()
                        .map_err(|e| {
                            CodegenError::FailedToParseArgVar(name.to_owned(), e.to_string())
                        })
                        .into_diagnostic()?,
                    None => var.default as i32,
                };

                if args.defaults || var.prompt.is_none() {
                    log_var(name, &default, Some(default_comment));

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
                        .into_diagnostic()?;

                    log_var(name, &value, None);

                    context.insert(name, &value);
                }
            }
            TemplateVariable::String(var) => {
                let required = var.required.unwrap_or_default();
                let default = custom_vars.get(name).unwrap_or(&var.default);

                if args.defaults || var.prompt.is_none() {
                    log_var(name, &default, Some(default_comment));

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
                        .into_diagnostic()?;

                    log_var(name, &value, None);

                    context.insert(name, &value);
                }
            }
        }
    }

    Ok(context)
}

pub async fn generate(args: GenerateArgs) -> AppResult {
    let workspace = load_workspace().await?;
    let generator = CodeGenerator::new(&workspace.root, &workspace.config.generator);
    let theme = create_theme();
    let cwd = env::current_dir().into_diagnostic()?;

    // This is a special case for creating a new template with the generator itself!
    if args.template {
        let template = generator.create_template(&args.name)?;

        println!(
            "Created a new template {} at {}",
            color::id(template.id),
            color::path(template.root)
        );

        return Ok(());
    }

    if args.dry_run {
        debug!("Running in DRY MODE");
    }

    // Create the template instance
    let mut template = generator.load_template(&args.name)?;
    let term = Term::buffered_stdout();

    term.line("")?;
    term.line(format!(
        "{} {}",
        &template.config.title,
        if args.dry_run {
            color::muted("(dry run)")
        } else {
            "".into()
        }
    ))?;
    term.line(&template.config.description)?;
    term.line("")?;
    term.flush_lines()?;

    // Determine the destination path
    let relative_dest = RelativePathBuf::from(match &args.dest {
        Some(d) => d.clone(),
        None => {
            debug!("Destination path not provided, prompting the user");

            Input::with_theme(&theme)
                .with_prompt("Where to generate code to?")
                .allow_empty(false)
                .interact_text()
                .into_diagnostic()?
        }
    });
    let dest = relative_dest.to_logical_path(&cwd);

    debug!(dest = ?dest, "Destination path set");

    // Gather variables and build context
    let mut context = gather_variables(&template, &theme, &args)?;
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

    term.line("")?;

    for file in template.files {
        term.line(format!(
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
                    .strip_prefix(&cwd)
                    .unwrap_or(&file.dest_path)
                    .to_str()
                    .unwrap()
            )
        ))?;
    }

    term.line("")?;
    term.flush_lines()?;

    Ok(())
}
