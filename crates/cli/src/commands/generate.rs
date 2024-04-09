use crate::helpers::create_theme;
use clap::Args;
use dialoguer::{theme::Theme, Confirm, Input, MultiSelect, Select};
use itertools::Itertools;
use miette::IntoDiagnostic;
use moon_app_components::{Console, MoonEnv};
use moon_codegen::{
    parse_args_into_variables, CodeGenerator, CodegenError, FileState, Template, TemplateContext,
};
use moon_config::TemplateVariable;
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use starbase::{system, AppResult};
use starbase_styles::color;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
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

fn log_var<T: Debug>(name: &str, value: &T, comment: Option<&str>) {
    debug!(name, value = ?value, comment, "Setting variable");
}

fn parse_var_args(
    vars: &[String],
    config: &FxHashMap<String, TemplateVariable>,
) -> FxHashMap<String, String> {
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

        // Skip if an internal variable
        if let Some(var_config) = config.get(name) {
            if var_config.is_internal() {
                return;
            }
        }

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

    dbg!(parse_args_into_variables(
        &args.vars,
        &template.config.variables
    )?);

    let custom_vars = parse_var_args(&args.vars, &template.config.variables);
    let default_comment = "(defaults)";

    debug!("Declaring variable values from defaults and user prompts");

    let mut variables = template.config.variables.iter().collect_vec();

    // Sort variables so prompting happens in the correct order
    variables.sort_by(|a, d| a.1.get_order().cmp(&d.1.get_order()));

    for (name, config) in variables {
        match config {
            TemplateVariable::Boolean(var) => {
                let default: bool = match custom_vars.get(name) {
                    Some(val) => val == "true" || val == "1" || val == "on",
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
                let values = var.get_values();
                let labels = var.get_labels();

                let defaults = custom_vars
                    .get(name)
                    .map(|v| vec![v])
                    .unwrap_or(var.default.to_vec());

                let default_index = values
                    .iter()
                    .position(|v| defaults.contains(v))
                    .unwrap_or_default();

                if args.defaults {
                    log_var(name, &defaults, Some(default_comment));
                }

                match (
                    args.defaults || var.prompt.is_none(),
                    var.multiple.unwrap_or_default(),
                ) {
                    (true, true) => {
                        context.insert(name, &defaults);
                    }
                    (true, false) => {
                        // Default value may not be defined, but for enums,
                        // we should always have a value when not multiple
                        context.insert(name, &values[default_index]);
                    }
                    (false, true) => {
                        let indexes = MultiSelect::with_theme(theme)
                            .with_prompt(var.prompt.as_ref().unwrap())
                            .items(&labels)
                            .defaults(
                                &values
                                    .iter()
                                    .map(|v| defaults.contains(v))
                                    .collect::<Vec<bool>>(),
                            )
                            .interact()
                            .into_diagnostic()?;
                        let value = indexes
                            .iter()
                            .map(|i| values[*i].clone())
                            .collect::<Vec<String>>();

                        log_var(name, &value, None);

                        context.insert(name, &value);
                    }
                    (false, false) => {
                        let index = Select::with_theme(theme)
                            .with_prompt(var.prompt.as_ref().unwrap())
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

#[system]
pub async fn generate(
    args: ArgsRef<GenerateArgs>,
    workspace: ResourceRef<Workspace>,
    console: ResourceRef<Console>,
    moon_env: StateRef<MoonEnv>,
) {
    let mut generator = CodeGenerator::new(
        &workspace.root,
        &workspace.config.generator,
        Arc::clone(moon_env),
    );
    let console = console.stdout();
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
    let mut context = gather_variables(&template, &theme, args)?;

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
    let dest = relative_dest.to_logical_path(&workspace.working_dir);

    debug!(dest = ?dest, "Destination path set");

    // Inject built-in context variables
    context.insert("dest_dir", &dest);
    context.insert("dest_rel_dir", &relative_dest);
    context.insert("working_dir", &workspace.working_dir);
    context.insert("workspace_root", &workspace.root);

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
                    .strip_prefix(&workspace.working_dir)
                    .unwrap_or(&file.dest_path)
                    .to_string_lossy()
            )
        ))?;
    }

    console.write_newline()?;
    console.flush()?;
}
