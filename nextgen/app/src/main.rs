mod app_error;
mod systems;

use starbase::tracing::TracingOptions;
use starbase::{App, MainResult};
use starbase_utils::string_vec;
use systems::find_workspace_root;

#[tokio::main]
async fn main() -> MainResult {
    App::setup_diagnostics();

    App::setup_tracing_with_options(TracingOptions {
        filter_modules: string_vec!["moon", "proto", "schematic", "starbase"],
        log_env: "STARBASE_LOG".into(),
        // log_file: args.log_file,
        test_env: "MOON_TEST".into(),
        ..TracingOptions::default()
    });

    let mut app = App::new();
    app.startup(find_workspace_root);
    app.run().await?;

    Ok(())
}
