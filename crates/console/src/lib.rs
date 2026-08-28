mod reporter;
mod theme;

use async_trait::async_trait;
use iocraft::prelude::*;

pub use reporter::*;
pub use starbase_console::{ConsoleError, ConsoleStream, ui};
pub use theme::*;

pub type Console = starbase_console::Console<MoonReporter>;

#[async_trait]
pub trait ConsoleExt {
    async fn render_prompt<T: Component>(
        &self,
        element: Element<'_, T>,
    ) -> Result<(), ConsoleError>;
}

#[async_trait]
impl ConsoleExt for Console {
    async fn render_prompt<T: Component>(
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
