mod reporter;
mod theme;

pub use reporter::*;
pub use starbase_console::ui;
pub use theme::*;

use starbase_console::Console;

pub type MoonConsole = Console<MoonReporter>;
