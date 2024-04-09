use std::io::{stdout, IsTerminal};

use crate::codegen_error::CodegenError;
use crate::template::Template;
use clap::builder::{
    BoolValueParser, PossibleValuesParser, RangedI64ValueParser, StringValueParser,
};
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, Args, Command};
use moon_config::TemplateVariable;
use moon_console::prompts::Confirm;
use rustc_hash::FxHashMap;
use tera::Context as TemplateContext;
use tracing::debug;

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

    #[arg(help = "Run entire generator process without writing files")]
    dry_run: bool,

    #[arg(long, help = "Force overwrite any existing files at the destination")]
    force: bool,

    #[arg(long, help = "Create a new template")]
    template: bool,

    // Variable args (after --)
    #[arg(last = true, help = "Arguments to define as variable values")]
    vars: Vec<String>,
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
                                debug!(var = name, value, "Setting boolean variable");

                                vars.insert(name, value);
                            }
                        }
                    }
                    TemplateVariable::Enum(inner) => {
                        if inner.is_multiple() {
                            if let Some(value) = matches.get_many::<String>(arg_name) {
                                let value = value.collect::<Vec<_>>();

                                debug!(
                                    var = name,
                                    value = ?value,
                                    "Setting multiple enum variable"
                                );

                                vars.insert(name, &value);
                            }
                        } else if let Some(value) = matches.get_one::<String>(arg_name) {
                            debug!(var = name, value, "Setting single enum variable");

                            vars.insert(name, value);
                        }
                    }
                    TemplateVariable::Number(_) => {
                        if let Some(value) = matches.get_one::<isize>(arg_name) {
                            debug!(var = name, value, "Setting number variable");

                            vars.insert(name, value);
                        }
                    }
                    TemplateVariable::String(_) => {
                        if let Some(value) = matches.get_one::<String>(arg_name) {
                            debug!(var = name, value, "Setting string variable");

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
                error: miette::miette!("{}", message.trim()).into(),
            }
            .into());
        }
    };

    Ok(vars)
}

pub fn gather_variables(
    args: &GenerateArgs,
    template: &Template,
) -> miette::Result<TemplateContext> {
    let mut context = parse_args_into_variables(&args.vars, &template.config.variables)?;

    debug!("Gathering variable values from defaults and user prompts");

    let mut variables = template.config.variables.iter().collect::<Vec<_>>();
    let skip_prompts = args.defaults || !stdout().is_terminal();

    // Sort variables so prompting happens in the correct order
    variables.sort_by(|a, d| a.1.get_order().cmp(&d.1.get_order()));

    for (name, config) in variables {
        match config {
            TemplateVariable::Boolean(cfg) => {
                if skip_prompts || cfg.prompt.is_none() {
                    if !context.contains_key(name) {
                        context.insert(name, &cfg.default);
                    }
                } else {
                }
            }
            _ => {}
        }
    }

    Ok(context)
}
