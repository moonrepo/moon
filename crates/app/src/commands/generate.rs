use crate::session::MoonSession;
use clap::Args;
use clap::builder::{
    BoolValueParser, PossibleValuesParser, RangedI64ValueParser, StringValueParser,
};
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, Command};
use iocraft::prelude::{View, Weight, element};
use moon_codegen::{CodeGenerator, CodegenError, FileState, Template};
use moon_config::{TemplateVariable, TemplateVariableEnumDefault};
use moon_console::{
    Console,
    ui::{
        Confirm, Container, Input, List, ListItem, Notice, Section, Select, SelectOption,
        StyledText, Variant,
    },
};
use rustc_hash::FxHashMap;
use starbase::AppResult;
use std::path::PathBuf;
use std::sync::Arc;
use tera::Context as TemplateContext;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    #[arg(help = "Name of template to generate")]
    name: String,

    #[arg(help = "Destination path, relative from workspace root or working directory")]
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

#[instrument(skip(config))]
pub fn parse_args_into_variables(
    args: &[String],
    config: &FxHashMap<String, TemplateVariable>,
) -> miette::Result<TemplateContext> {
    let mut vars = TemplateContext::default();

    if args.is_empty() {
        return Ok(vars);
    }

    debug!(args = ?args, "Inheriting variable values from command line arguments");

    // Create a clap command of arguments based on our config
    let command_name = "__generate__".to_owned();
    let mut command = Command::new(&command_name);

    for (name, cfg) in config {
        if cfg.is_internal() {
            continue;
        }

        match cfg {
            TemplateVariable::Boolean(_) => {
                command = command.arg(
                    Arg::new(name)
                        .long(name)
                        .action(ArgAction::SetTrue)
                        .value_parser(BoolValueParser::new()),
                );

                let negated_name = format!("no-{name}");

                command = command.arg(
                    Arg::new(&negated_name)
                        .long(negated_name)
                        .action(ArgAction::SetFalse)
                        .value_parser(BoolValueParser::new()),
                );
            }
            TemplateVariable::Enum(inner) => {
                command = command.arg(
                    Arg::new(name)
                        .long(name)
                        .action(if inner.is_multiple() {
                            ArgAction::Append
                        } else {
                            ArgAction::Set
                        })
                        .value_parser(PossibleValuesParser::new(inner.get_values())),
                );
            }
            TemplateVariable::Number(_) => {
                command = command.arg(
                    Arg::new(name)
                        .long(name)
                        .action(ArgAction::Set)
                        .value_parser(RangedI64ValueParser::<isize>::new())
                        .allow_negative_numbers(true),
                );
            }
            TemplateVariable::String(_) => {
                command = command.arg(
                    Arg::new(name)
                        .long(name)
                        .action(ArgAction::Set)
                        .value_parser(StringValueParser::new()),
                );
            }
        };
    }

    // Attempt to parse the arguments and extract matches
    let mut temp_args = vec![&command_name];
    temp_args.extend(args);

    match command.try_get_matches_from(temp_args) {
        Ok(matches) => {
            // Loop in order of arguments passed, instead of looping
            // the variable configs, so that subsequent values can overwrite
            for arg_id in matches.ids() {
                let arg_name = arg_id.as_str();
                let name = arg_name.strip_prefix("no-").unwrap_or(arg_name);

                let Some(cfg) = config.get(name) else {
                    continue;
                };

                if cfg.is_internal() {
                    continue;
                }

                match cfg {
                    TemplateVariable::Boolean(_) => {
                        // Booleans always have a value when matched, so only extract
                        // the value when it was actually passed on the command line
                        if let Some(ValueSource::CommandLine) = matches.value_source(arg_name) {
                            if let Some(value) = matches.get_one::<bool>(arg_name) {
                                debug!(name, value, "Setting boolean variable");

                                vars.insert(name, value);
                            }
                        }
                    }
                    TemplateVariable::Enum(inner) => {
                        if inner.is_multiple() {
                            if let Some(value) = matches.get_many::<String>(arg_name) {
                                let value = value.collect::<Vec<_>>();

                                debug!(
                                    name,
                                    value = ?value,
                                    "Setting multiple-value enum variable"
                                );

                                vars.insert(name, &value);
                            }
                        } else if let Some(value) = matches.get_one::<String>(arg_name) {
                            debug!(name, value, "Setting single-value enum variable");

                            vars.insert(name, value);
                        }
                    }
                    TemplateVariable::Number(_) => {
                        if let Some(value) = matches.get_one::<isize>(arg_name) {
                            debug!(name, value, "Setting number variable");

                            vars.insert(name, value);
                        }
                    }
                    TemplateVariable::String(_) => {
                        if let Some(value) = matches.get_one::<String>(arg_name) {
                            debug!(name, value, "Setting string variable");

                            vars.insert(name, value);
                        }
                    }
                };
            }
        }
        Err(error) => {
            let mut message = String::new();

            // clap includes some information we don't want,
            // so let's try and strip it before adding to the error
            for line in error.to_string().lines() {
                let line = line.strip_prefix("error:").unwrap_or(line);

                if line.is_empty()
                    || line.starts_with("Usage:")
                    || line.contains("For more information")
                {
                    continue;
                }

                message.push_str(line);
                message.push('\n');
            }

            return Err(CodegenError::FailedToParseArgs {
                error: miette::miette!("{}", message.trim()),
            }
            .into());
        }
    };

    Ok(vars)
}

#[instrument(skip(template, console))]
pub async fn gather_variables(
    args: &GenerateArgs,
    template: &Template,
    console: &Console,
) -> miette::Result<TemplateContext> {
    let mut context = parse_args_into_variables(&args.vars, &template.config.variables)?;

    debug!("Gathering variable values from defaults and user prompts");

    let mut variables = template.config.variables.iter().collect::<Vec<_>>();
    let skip_prompts = args.defaults || !console.out.is_terminal();

    // Sort variables so prompting happens in the correct order
    variables.sort_by(|a, d| a.1.get_order().cmp(&d.1.get_order()));

    for (name, config) in variables {
        if context.contains_key(name) {
            continue;
        }

        let required = config.is_required();

        match config {
            TemplateVariable::Boolean(cfg) => {
                let value = if skip_prompts || cfg.prompt.is_none() {
                    cfg.default
                } else {
                    let mut value = cfg.default;

                    console
                        .render_interactive(element! {
                            Confirm(
                                label: cfg.prompt.as_ref().unwrap(),
                                on_confirm: &mut value,
                            )
                        })
                        .await?;

                    value
                };

                debug!(name, value, "Setting boolean variable");

                context.insert(name, &value);
            }
            TemplateVariable::Number(cfg) => {
                let value = if skip_prompts || cfg.prompt.is_none() {
                    cfg.default
                } else {
                    let mut value = String::new();

                    console
                        .render_interactive(element! {
                            Input(
                                label: cfg.prompt.as_ref().unwrap(),
                                default_value: cfg.default.to_string(),
                                on_value: &mut value,
                                validate: move |input: String| {
                                    let Ok(number) = input.parse::<isize>() else {
                                        return Some("A number is required".into());
                                    };

                                    if required && number == 0 {
                                        Some("A non-zero value is required".into())
                                    } else {
                                        None
                                    }
                                }
                            )
                        })
                        .await?;

                    value.parse::<isize>().unwrap()
                };

                debug!(name, value, "Setting number variable");

                context.insert(name, &value);
            }
            TemplateVariable::String(cfg) => {
                let value = if skip_prompts || cfg.prompt.is_none() {
                    cfg.default.clone()
                } else {
                    let mut value = String::new();

                    console
                        .render_interactive(element! {
                            Input(
                                label: cfg.prompt.as_ref().unwrap(),
                                default_value: &cfg.default,
                                on_value: &mut value,
                                validate: move |input: String| {
                                    if required && input.is_empty() {
                                        Some("A value is required".into())
                                    } else {
                                        None
                                    }
                                }
                            )
                        })
                        .await?;

                    value
                };

                debug!(name, value, "Setting string variable");

                context.insert(name, &value);
            }
            TemplateVariable::Enum(cfg) if cfg.is_multiple() => {
                let values = cfg.get_values();
                let labels = cfg.get_labels();
                let default_value = match &cfg.default {
                    TemplateVariableEnumDefault::Vec(def) => def.to_owned(),
                    TemplateVariableEnumDefault::String(def) => vec![def.to_owned()],
                };

                let value = if skip_prompts || cfg.prompt.is_none() {
                    default_value
                } else {
                    let default_indexes = values
                        .iter()
                        .enumerate()
                        .filter_map(|(index, value)| {
                            if default_value.contains(value) {
                                Some(index)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    let mut indexes = vec![];

                    console
                        .render_interactive(element! {
                            Select(
                                label: cfg.prompt.as_ref().unwrap(),
                                multiple: true,
                                options: labels
                                    .iter()
                                    .map(SelectOption::new)
                                    .collect::<Vec<_>>(),
                                default_indexes,
                                on_indexes: &mut indexes,
                            )
                        })
                        .await?;

                    indexes
                        .into_iter()
                        .map(|index| values[index].clone())
                        .collect()
                };

                debug!(
                    name,
                    value = ?value,
                    "Setting multiple-value enum variable"
                );

                context.insert(name, &value);
            }
            TemplateVariable::Enum(cfg) if !cfg.is_multiple() => {
                let values = cfg.get_values();
                let labels = cfg.get_labels();
                let default_value = match &cfg.default {
                    TemplateVariableEnumDefault::String(def) => def.to_owned(),
                    TemplateVariableEnumDefault::Vec(def) => {
                        if def.is_empty() {
                            values[0].to_owned()
                        } else {
                            def[0].to_owned()
                        }
                    }
                };

                let value = if skip_prompts || cfg.prompt.is_none() {
                    default_value
                } else {
                    let mut index = 0;

                    console
                        .render_interactive(element! {
                            Select(
                                label: cfg.prompt.as_ref().unwrap(),
                                options: labels
                                    .iter()
                                    .map(SelectOption::new)
                                    .collect::<Vec<_>>(),
                                on_index: &mut index,
                            )
                        })
                        .await?;

                    values[index].to_owned()
                };

                debug!(name, value, "Setting single-value enum variable");

                context.insert(name, &value);
            }
            _ => {
                // Rust can't infer enum variants above correctly!
            }
        }
    }

    Ok(context)
}

#[instrument(skip_all)]
pub async fn generate(session: MoonSession, args: GenerateArgs) -> AppResult {
    let mut generator = CodeGenerator::new(
        &session.workspace_root,
        &session.workspace_config.generator,
        Arc::clone(&session.moon_env),
    );
    let console = &session.console;

    // This is a special case for creating a new template with the generator itself!
    if args.template {
        let template = generator.create_template(&args.name)?;

        console.render(element! {
            Container {
                Notice(variant: Variant::Success) {
                    StyledText(content: format!(
                        "Created a new template <id>{}</id> at <path>{}</path>",
                        template.id,
                        template.root.display(),
                    ))
                }
            }
        })?;

        return Ok(None);
    }

    if args.dry_run {
        debug!("Running in DRY MODE");
    }

    generator.load_templates().await?;

    // Create the template instance
    let mut template = generator.get_template(&args.name)?;
    let mut has_prompts = !template.config.variables.is_empty();

    console.render(element! {
        Container{
            Section(title: template.id.as_str()) {
                StyledText(
                    content: if args.dry_run {
                        format!("{} <invalid>(dry run)</invalid>", template.config.title)
                    } else {
                        template.config.title.clone()
                    },
                    weight: Weight::Bold
                )
                StyledText(content: &template.config.description)
            }
        }
    })?;

    // Gather variables
    let mut context = gather_variables(&args, &template, &session.console).await?;
    context.insert("working_dir", &session.working_dir);
    context.insert("workspace_root", &session.workspace_root);

    // Determine the destination path
    let relative_dest = match &args.dest {
        Some(dest) => dest.to_owned(),
        None => {
            if let Some(dest) = &template.config.destination {
                debug!(dest, "Default destination path provided by template config");

                dest.to_owned()
            } else {
                debug!("Destination path not provided, prompting the user");

                let mut value = String::new();

                console
                    .render_interactive(element! {
                        Input(
                            label: "Where to generate code to?",
                            description: "Relative from the current directory.".to_owned(),
                            on_value: &mut value,
                            validate: move |input: String| {
                                if input.is_empty() {
                                    Some("Please provide a relative path".into())
                                } else {
                                    None
                                }
                            }
                        )
                    })
                    .await?;

                value
            }
        }
    };
    let relative_from_root = relative_dest.starts_with('/');
    let relative_dest = template.interpolate_path(&PathBuf::from(relative_dest), &context)?;
    let dest = relative_dest.to_logical_path(if relative_from_root {
        &session.workspace_root
    } else {
        &session.working_dir
    });

    debug!(dest = ?dest, "Destination path set");

    // Inject built-in context variables
    context.insert("dest_dir", &dest);
    context.insert("dest_rel_dir", &relative_dest);

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
                let mut index = 2;
                has_prompts = true;

                console
                    .render_interactive(element! {
                        Select(
                            label: format!(
                                "File <path>{}</path> already exists, what to do?",
                                file.dest_path.display()
                            ),
                            default_index: 2,
                            on_index: &mut index,
                            options: vec![
                                SelectOption::new("Keep existing file"),
                                SelectOption::new("Merge new file into existing file"),
                                SelectOption::new("Replace existing with new file"),
                            ]
                        )
                    })
                    .await?;

                file.state = match index {
                    0 => FileState::Skip,
                    1 => FileState::Merge,
                    _ => FileState::Replace,
                };

                continue;
            }

            // Confirm whether to replace file
            let mut confirmed = false;
            has_prompts = true;

            console
                .render_interactive(element! {
                    Confirm(
                        label: format!(
                            "File <path>{}</path> already exists, overwrite?",
                            file.dest_path.display()
                        ),
                        on_confirm: &mut confirmed
                    )
                })
                .await?;

            if confirmed {
                file.state = FileState::Replace;
            }
        }
    }

    // Generate the files in the destination and print the results
    if !args.dry_run {
        generator.generate(&template)?;
    }

    console.render(element! {
        View(
            margin_top: if has_prompts {
                1
            } else {
                0
            },
            margin_bottom: 1
        ) {
            List {
                #(template.files.values().map(|file| {
                    let (label, arrow, style) = match &file.state {
                        FileState::Create => ("created", "--➤", "success"),
                        FileState::Merge => ("merged", "---➤", "success"),
                        FileState::Replace => ("replaced", "-➤", "failure"),
                        FileState::Skip => ("skipped", "--➤", "invalid"),
                    };

                    element! {
                        ListItem {
                            StyledText(
                                content: format!(
                                    "<{style}>{label}</{style}> <muted>{arrow}</muted> <mutedlight>{}</mutedlight>",
                                    file.dest_path
                                        .strip_prefix(&session.working_dir)
                                        .unwrap_or(&file.dest_path)
                                        .display()
                                ),
                            )
                        }
                    }
                }))

                #(template.assets.values().map(|asset| {
                    element! {
                        ListItem {
                            StyledText(
                                content: format!(
                                    "<success>created</success> <muted>--➤</muted> <mutedlight>{}</mutedlight>",
                                    asset.dest_path
                                        .strip_prefix(&session.working_dir)
                                        .unwrap_or(&asset.dest_path)
                                        .display()
                                ),
                            )
                        }
                    }
                }))
            }
        }
    })?;

    Ok(None)
}
