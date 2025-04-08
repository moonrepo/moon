use crate::workspace_mocker::WorkspaceMocker;
use moon_app_context::AppContext;
use moon_console::{Console, MoonReporter};
use std::path::Path;

pub fn create_test_console() -> Console {
    let mut console = Console::new_testing();
    console.set_reporter(MoonReporter::default());
    console
}

pub fn generate_app_context_from_sandbox(root: &Path) -> AppContext {
    WorkspaceMocker::new(root)
        .load_default_configs()
        .mock_app_context()
}
