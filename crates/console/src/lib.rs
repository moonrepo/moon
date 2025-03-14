// mod buffer;
// mod console;
// mod printer;
// pub mod prompts;
mod default_reporter;
mod reporter;

// pub use buffer::*;
// pub use console::*;
// pub use printer::*;
pub use default_reporter::*;
pub use reporter::*;
pub use starbase_console::{theme, ui};

use starbase_console::Console;

pub type MoonConsole = Console<DefaultReporter>;
