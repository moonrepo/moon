mod app_error;
mod commands;
mod components;
mod queries;
mod session;
mod systems;

use session::CliSession;
use starbase::tracing::TracingOptions;
use starbase::{App, MainResult};
use starbase_utils::string_vec;

#[tokio::main]
async fn main() -> MainResult {
    let app = App::default();
    app.setup_diagnostics();

    let _guard = app.setup_tracing(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "schematic", "starbase", "warpgate"],
        log_env: "MOON_LOG".into(),
        // log_file: cli.log_file.clone(),
        // test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

    let mut session = CliSession::new();

    app.run(&mut session, |s| async move {
        dbg!(&s);
        println!("Hello");

        Ok(())
    })
    .await?;

    Ok(())
}
