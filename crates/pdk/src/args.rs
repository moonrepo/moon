pub use clap;
pub use clap::Args;

use clap::{Parser, Subcommand};
use warpgate_pdk::AnyResult;

#[derive(Subcommand)]
enum ArgsCommand<T: Args> {
    #[command(name = "__internal__", rename_all = "kebab-case")]
    Internal(T),
}

#[derive(Parser)]
#[command(
    disable_colored_help = true,
    disable_help_flag = true,
    disable_help_subcommand = true,
    disable_version_flag = true,
    ignore_errors = false,
    no_binary_name = true,
    propagate_version = false
)]
struct ArgsParser<T: Args> {
    #[command(subcommand)]
    command: ArgsCommand<T>,
}

/// Parse the list of argument strings into flags, options, and
/// positionals, and assign the values to the provided [`Args`] struct.
pub fn parse_args<T: Args>(args: &[String]) -> AnyResult<T> {
    let internal_command = String::from("__internal__");

    let mut internal_args = vec![&internal_command];
    internal_args.extend(args);

    let parsed = ArgsParser::<T>::try_parse_from(internal_args)?;
    let ArgsCommand::Internal(args) = parsed.command;

    Ok(args)
}
