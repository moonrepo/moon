use crate::app::App;
use crate::session::CliSession;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use miette::miette;
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<Shell>,
}

#[instrument(skip_all)]
pub async fn completions(session: CliSession, args: CompletionsArgs) -> AppResult {
    let Some(shell) = args.shell.or_else(Shell::from_env) else {
        return Err(miette!(
            code = "moon::completions",
            "Could not determine your shell!"
        ));
    };

    session.console.quiet();

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "moon", &mut stdio);

    Ok(())
}
