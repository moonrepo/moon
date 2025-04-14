use crate::app::Cli;
use crate::session::MoonSession;
use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};
use clap_complete_nushell::Nushell;
use miette::IntoDiagnostic;
use starbase::AppResult;
use starbase_shell::ShellType;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct CompletionsArgs {
    #[arg(long, help = "Shell to generate for")]
    shell: Option<ShellType>,
}

#[instrument(skip_all)]
pub async fn completions(session: MoonSession, args: CompletionsArgs) -> AppResult {
    let shell = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect().into_diagnostic()?,
    };

    session.console.quiet();

    let mut app = Cli::command();
    let mut stdio = std::io::stdout();

    let clap_shell = match shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Pwsh => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Nu => {
            generate(Nushell, &mut app, "moon", &mut stdio);

            return Ok(None);
        }
        unsupported => {
            eprintln!("{unsupported} does not currently support completions");

            return Ok(Some(1));
        }
    };

    generate(clap_shell, &mut app, "moon", &mut stdio);

    Ok(None)
}
