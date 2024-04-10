use crate::codegen_error::CodegenError;
use crate::template::Template;
use clap::builder::{
    BoolValueParser, PossibleValuesParser, RangedI64ValueParser, StringValueParser,
};
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, Args, Command};
use moon_config::{TemplateVariable, TemplateVariableEnumDefault};
use moon_console::prompts::list_option::ListOption;
use moon_console::prompts::validator::Validation;
use moon_console::prompts::{Confirm, CustomType, MultiSelect, Select, Text};
use moon_console::Console;
use rustc_hash::FxHashMap;
use std::io::{stdout, IsTerminal};
use tera::Context as TemplateContext;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct GenerateArgs {
    #[arg(help = "Name of template to generate")]
    pub name: String,

    #[arg(help = "Destination path, relative from the current working directory")]
    pub dest: Option<String>,

    #[arg(
        long,
        help = "Use the default value of all variables instead of prompting"
    )]
    pub defaults: bool,

    #[arg(
        long = "dryRun",
        help = "Run entire generator process without writing files"
    )]
    pub dry_run: bool,

    #[arg(long, help = "Force overwrite any existing files at the destination")]
    pub force: bool,

    #[arg(long, help = "Create a new template")]
    pub template: bool,

    // Variable args (after --)
    #[arg(last = true, help = "Arguments to define as variable values")]
    pub vars: Vec<String>,
}

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

pub fn gather_variables(
    args: &GenerateArgs,
    template: &Template,
    console: &Console,
) -> miette::Result<TemplateContext> {
    let mut context = parse_args_into_variables(&args.vars, &template.config.variables)?;

    debug!("Gathering variable values from defaults and user prompts");

    let mut variables = template.config.variables.iter().collect::<Vec<_>>();
    let skip_prompts = args.defaults || !stdout().is_terminal();

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
                    console.confirm(
                        Confirm::new(cfg.prompt.as_ref().unwrap()).with_default(cfg.default),
                    )?
                };

                debug!(name, value, "Setting boolean variable");

                context.insert(name, &value);
            }
            TemplateVariable::Number(cfg) => {
                let value = if skip_prompts || cfg.prompt.is_none() {
                    cfg.default
                } else {
                    console.prompt_custom(
                        CustomType::<isize>::new(cfg.prompt.as_ref().unwrap())
                            .with_default(cfg.default)
                            .with_validator(move |input: &isize| {
                                if required && *input == 0 {
                                    Ok(Validation::Invalid("A non-zero value is required".into()))
                                } else {
                                    Ok(Validation::Valid)
                                }
                            }),
                    )?
                };

                debug!(name, value, "Setting number variable");

                context.insert(name, &value);
            }
            TemplateVariable::String(cfg) => {
                let value = if skip_prompts || cfg.prompt.is_none() {
                    cfg.default.clone()
                } else {
                    console.prompt_text(
                        Text::new(cfg.prompt.as_ref().unwrap())
                            .with_default(&cfg.default)
                            .with_validator(move |input: &str| {
                                if required && input.is_empty() {
                                    Ok(Validation::Invalid("A value is required".into()))
                                } else {
                                    Ok(Validation::Valid)
                                }
                            }),
                    )?
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

                    let selected = console.prompt_multiselect(
                        MultiSelect::new(
                            cfg.prompt.as_ref().unwrap(),
                            labels
                                .iter()
                                .enumerate()
                                .map(|(index, label)| ListOption::new(index, label))
                                .collect::<Vec<_>>(),
                        )
                        .with_default(&default_indexes),
                    )?;

                    selected
                        .into_iter()
                        .map(|option| values[option.index].clone())
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
                    let selected = console.prompt_select(Select::new(
                        cfg.prompt.as_ref().unwrap(),
                        labels
                            .iter()
                            .enumerate()
                            .map(|(index, label)| ListOption::new(index, label))
                            .collect::<Vec<_>>(),
                    ))?;

                    values[selected.index].to_owned()
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
