use crate::app::App;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use miette::miette;
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<Shell>,
}

#[system]
pub async fn completions(args: ArgsRef<CompletionsArgs>) {
    let Some(shell) = args.shell.or_else(Shell::from_env) else {
        return Err(miette!("Could not determine your shell!"));
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "moon", &mut stdio);
}
