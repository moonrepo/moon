mod reporter;
mod theme;

use iocraft::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

pub use reporter::*;
pub use starbase_console::ConsoleError;
pub use starbase_console::ui;
pub use theme::*;

pub type MoonConsole = starbase_console::Console<MoonReporter>;

#[derive(Clone, Debug)]
pub struct Console(MoonConsole);

impl Console {
    pub fn new(quiet: bool) -> Self {
        Self(MoonConsole::new(quiet))
    }

    pub fn new_testing() -> Self {
        Self(MoonConsole::new_testing())
    }

    pub async fn render_prompt<T: Component>(
        &self,
        element: Element<'_, T>,
    ) -> Result<(), ConsoleError> {
        self.render_interactive_with_options(
            element,
            ui::RenderOptions {
                handle_interrupt: true,
                ignore_ctrl_c: true,
                ..Default::default()
            },
        )
        .await
    }
}

impl Deref for Console {
    type Target = MoonConsole;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Console {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
