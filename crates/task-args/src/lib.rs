use moon_args::split_args;
use moon_config::TaskArgs;

pub fn parse_task_args(args: &TaskArgs) -> miette::Result<Vec<String>> {
    Ok(match args {
        TaskArgs::None => vec![],
        TaskArgs::String(cmd_string) => split_args(cmd_string)?,
        TaskArgs::List(cmd_args) => cmd_args.to_owned(),
    })
}
