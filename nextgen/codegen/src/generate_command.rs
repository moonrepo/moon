use clap::builder::{
    BoolValueParser, PossibleValuesParser, RangedI64ValueParser, StringValueParser,
};
use clap::{Arg, ArgAction, Command};
use moon_config::TemplateVariable;
use rustc_hash::FxHashMap;
use tera::Context as TemplateContext;
use tracing::debug;

pub fn parse_args_into_variables(
    args: &[String],
    config: &FxHashMap<String, TemplateVariable>,
) -> TemplateContext {
    let mut vars = TemplateContext::default();

    if args.is_empty() {
        return vars;
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

    if let Ok(matches) = command.try_get_matches_from(temp_args) {
        for (name, cfg) in config {
            if cfg.is_internal() {
                continue;
            }

            let arg_name = format!("--{name}");

            match cfg {
                TemplateVariable::Boolean(_) => {
                    let negated_name = format!("no-{name}");
                    let negated_arg_name = format!("--{negated_name}");

                    if let Some(value) = matches.get_one::<bool>(&negated_name) {
                        debug!(
                            arg = negated_arg_name,
                            var = name,
                            value,
                            "Setting boolean variable"
                        );

                        vars.insert(name, value);
                    } else if let Some(value) = matches.get_one::<bool>(name) {
                        debug!(
                            arg = arg_name,
                            var = name,
                            value,
                            "Setting boolean variable"
                        );

                        vars.insert(name, value);
                    }
                }
                TemplateVariable::Enum(inner) => {
                    if inner.is_multiple() {
                        if let Some(value) = matches.get_many::<String>(name) {
                            let value = value.collect::<Vec<_>>();

                            debug!(
                                arg = arg_name,
                                var = name,
                                value = ?value,
                                "Setting multiple enum variable"
                            );

                            vars.insert(name, &value);
                        }
                    } else if let Some(value) = matches.get_one::<String>(name) {
                        debug!(
                            arg = arg_name,
                            var = name,
                            value,
                            "Setting single enum variable"
                        );

                        vars.insert(name, value);
                    }
                }
                TemplateVariable::Number(_) => {
                    if let Some(value) = matches.get_one::<isize>(name) {
                        debug!(arg = arg_name, var = name, value, "Setting number variable");

                        vars.insert(name, value);
                    }
                }
                TemplateVariable::String(_) => {
                    if let Some(value) = matches.get_one::<String>(name) {
                        debug!(arg = arg_name, var = name, value, "Setting string variable");

                        vars.insert(name, value);
                    }
                }
            };
        }
    }

    vars
}
