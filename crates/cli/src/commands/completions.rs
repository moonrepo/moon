use crate::app::App;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use starbase::AppResult;

pub async fn completions(shell: Option<Shell>) -> AppResult {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err("Could not determine your shell!".into());
    };

    let mut app = App::command();
    let mut stdio = std::io::stdout();

    generate(shell, &mut app, "moon", &mut stdio);

    Ok(())
}
