use crate::tasks_builder_error::TasksBuilderError;
use moon_config::{EnvMap, TaskArgs};
use moon_target::Target;
use moon_task::TaskArg;
use rustc_hash::FxBuildHasher;

/// The result of parsing a task's `command` or `args` setting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandLineParseResult {
    pub command: Option<TaskArg>,
    pub args: Option<Vec<TaskArg>>,
    pub env: Option<EnvMap>,
    pub requires_shell: bool,
}

/// Parse a task's `command` or `args` setting into a list of arguments,
/// extracting leading env vars, and detecting whether a shell is required.
pub fn parse_command_line(
    target: &Target,
    args: &TaskArgs,
) -> miette::Result<CommandLineParseResult> {
    let mut res = CommandLineParseResult::default();

    match args {
        TaskArgs::Noop => Ok(res),
        TaskArgs::List(list) => {
            res.args = Some(list.iter().map(TaskArg::new).collect());

            Ok(res)
        }
        TaskArgs::String(cmd) => {
            use starbase_args::*;

            if cmd.is_empty() {
                return Ok(res);
            }

            let mut args = vec![];
            let mut env = EnvMap::default();

            let mut handle_arg = |arg: &Argument, extract_env: bool| {
                if let Argument::Value(value) = arg {
                    if !res.requires_shell
                        && matches!(
                            value,
                            Value::Expansion(_)
                                | Value::Substitution(_)
                                | Value::MurexBraceQuoted(_)
                                | Value::NuRawQuoted(_)
                        )
                    {
                        res.requires_shell = true;
                    }

                    if value.is_quoted() {
                        args.push(TaskArg::new_quoted(
                            value.get_quoted_value(),
                            value.to_string(),
                        ));
                    } else {
                        args.push(TaskArg::new_unquoted(value.to_string()));
                    }
                } else if let Argument::EnvVar(key, value, _) = arg {
                    if extract_env {
                        if !matches!(value, Value::Expansion(_) | Value::Substitution(_)) {
                            env.insert(key.to_owned(), Some(value.as_str().to_owned()));
                        }
                    } else {
                        args.push(TaskArg::new_unquoted(arg.to_string()));
                    }
                } else {
                    args.push(TaskArg::new_unquoted(arg.to_string()));
                }
            };

            let command_line =
                parse(cmd).map_err(|error| TasksBuilderError::InvalidCommandSyntax {
                    task: target.to_owned(),
                    command: cmd.to_owned(),
                    position: match error.line_col {
                        LineColLocation::Pos((line, col)) => format!("{line}:{col}"),
                        LineColLocation::Span((line, col), _) => format!("{line}:{col}"),
                    },
                })?;

            for pipeline in command_line.iter() {
                match pipeline {
                    Pipeline::Start(commands) => {
                        let mut allow_next_sequence = true;

                        for sequence in commands.iter() {
                            match sequence {
                                // If only env vars, allow it, otherwise it's a
                                // multi-command and we shouldn't allow it
                                Sequence::Start(command) | Sequence::Then(command) => {
                                    if !allow_next_sequence {
                                        return Err(TasksBuilderError::UnsupportedCommandSyntax {
                                            task: target.to_owned(),
                                        }
                                        .into());
                                    }

                                    allow_next_sequence = command
                                        .iter()
                                        .all(|arg| matches!(arg, Argument::EnvVar(_, _, _)));
                                    let mut extract_env = true;

                                    for arg in command.iter() {
                                        handle_arg(arg, extract_env);

                                        if extract_env && !matches!(arg, Argument::EnvVar(_, _, _))
                                        {
                                            extract_env = false;
                                        }
                                    }
                                }
                                // Capture anything after `--`
                                Sequence::Passthrough(command) => {
                                    handle_arg(
                                        &Argument::Value(Value::Unquoted("--".into())),
                                        false,
                                    );

                                    for arg in command.iter() {
                                        handle_arg(arg, false);
                                    }
                                }
                                Sequence::Stop(term) => {
                                    if term == "--" {
                                        handle_arg(
                                            &Argument::Value(Value::Unquoted("--".into())),
                                            false,
                                        );
                                    } else if term != ";" && term != "\n" {
                                        return Err(TasksBuilderError::UnsupportedCommandSyntax {
                                            task: target.to_owned(),
                                        }
                                        .into());
                                    }
                                }
                                _ => {
                                    return Err(TasksBuilderError::UnsupportedCommandSyntax {
                                        task: target.to_owned(),
                                    }
                                    .into());
                                }
                            };
                        }
                    }
                    _ => {
                        return Err(TasksBuilderError::UnsupportedCommandSyntax {
                            task: target.to_owned(),
                        }
                        .into());
                    }
                };
            }

            if !args.is_empty() {
                res.args = Some(args);
            }

            if !env.is_empty() {
                res.env = Some(env);
            }

            Ok(res)
        }
    }
}

/// A thread-safe cache of parsed command line strings, shared across all
/// projects (and their tasks) within a single graph build. Global tasks
/// are inherited by every project with the same `command` and `args`,
/// so this avoids re-parsing an identical string for each of them.
///
/// Only successful parses are cached, as errors embed the task target.
#[derive(Debug, Default)]
pub struct CommandLineCache {
    cache: scc::HashMap<String, CommandLineParseResult, FxBuildHasher>,
}

impl CommandLineCache {
    /// Parse the provided args, returning a cached result for an
    /// identical command line string that was previously parsed.
    pub fn parse(
        &self,
        target: &Target,
        args: &TaskArgs,
    ) -> miette::Result<CommandLineParseResult> {
        // Only strings require the pest parser, the others are cheap
        let TaskArgs::String(cmd) = args else {
            return parse_command_line(target, args);
        };

        if let Some(result) = self
            .cache
            .read_sync(cmd.as_str(), |_, result| result.clone())
        {
            return Ok(result);
        }

        // Parse outside of the bucket lock, as another thread may race
        // us for the same string, but the result is identical either way
        let result = parse_command_line(target, args)?;

        let _ = self.cache.insert_sync(cmd.to_owned(), result.clone());

        Ok(result)
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }
}
