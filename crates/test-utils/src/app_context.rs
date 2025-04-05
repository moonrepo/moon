use crate::app_context_mocker::AppContextMocker;
use moon_app_context::AppContext;
use moon_console::{Console, MoonReporter};
use starbase_sandbox::create_sandbox;
use std::path::Path;

pub fn create_console() -> Console {
    let mut console = Console::new_testing();
    console.set_reporter(MoonReporter::default());
    console
}

pub fn generate_app_context(fixture: &str) -> AppContext {
    generate_app_context_from_sandbox(create_sandbox(fixture).path())
}

pub fn generate_app_context_from_sandbox(root: &Path) -> AppContext {
    let mut mocker = AppContextMocker::new(root);
    mocker.load_root_configs();
    mocker.mock()
}
