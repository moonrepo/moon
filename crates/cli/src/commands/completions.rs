use crate::{app::App, helpers::AnyError};
use clap::CommandFactory;
use clap_complete::{generate, Shell};

pub async fn completions(shell: Option<Shell>) -> Result<(), AnyError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err("Could not determine your shell!".into());
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "moon", &mut stdio);

    Ok(())
}
