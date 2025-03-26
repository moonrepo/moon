mod reporter;
mod theme;

pub use reporter::*;
pub use starbase_console::ConsoleError;
pub use starbase_console::ui;
pub use theme::*;

pub type Console = starbase_console::Console<MoonReporter>;
