use crate::buffer::*;
use crate::prompts::create_theme;
use crate::reporter::*;
use inquire::ui::RenderConfig;
use moon_common::is_formatted_output;
use starbase::Resource;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type ConsoleTheme = RenderConfig<'static>;

#[derive(Clone, Resource)]
pub struct Console {
    pub err: Arc<ConsoleBuffer>,
    pub out: Arc<ConsoleBuffer>,

    quiet: Arc<AtomicBool>,
    reporter: Arc<BoxedReporter>,
    theme: Arc<ConsoleTheme>,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        let quiet = Arc::new(AtomicBool::new(quiet || is_formatted_output()));

        let mut err = ConsoleBuffer::new(ConsoleStream::Stderr);
        err.quiet = Some(Arc::clone(&quiet));

        let mut out = ConsoleBuffer::new(ConsoleStream::Stdout);
        out.quiet = Some(Arc::clone(&quiet));

        Self {
            err: Arc::new(err),
            out: Arc::new(out),
            quiet,
            reporter: Arc::new(Box::new(EmptyReporter)),
            theme: Arc::new(create_theme()),
        }
    }

    pub fn new_testing() -> Self {
        Self {
            err: Arc::new(ConsoleBuffer::new_testing(ConsoleStream::Stderr)),
            out: Arc::new(ConsoleBuffer::new_testing(ConsoleStream::Stdout)),
            quiet: Arc::new(AtomicBool::new(false)),
            reporter: Arc::new(Box::new(EmptyReporter)),
            theme: Arc::new(ConsoleTheme::empty()),
        }
    }

    pub fn close(&mut self) -> miette::Result<()> {
        dbg!(
            "ERR",
            Arc::weak_count(&self.err),
            Arc::strong_count(&self.err)
        );
        dbg!(
            "OUT",
            Arc::weak_count(&self.out),
            Arc::strong_count(&self.out)
        );

        if let Some(err) = Arc::get_mut(&mut self.err) {
            dbg!("STDERR");
            err.close()?;
        }

        if let Some(out) = Arc::get_mut(&mut self.out) {
            dbg!("STDOUT");
            out.close()?;
        }

        Ok(())
    }

    pub fn quiet(&self) {
        self.quiet.store(true, Ordering::Release);
    }

    pub fn stderr(&self) -> Arc<ConsoleBuffer> {
        Arc::clone(&self.err)
    }

    pub fn stdout(&self) -> Arc<ConsoleBuffer> {
        Arc::clone(&self.out)
    }

    pub fn theme(&self) -> Arc<ConsoleTheme> {
        Arc::clone(&self.theme)
    }

    pub fn with_reporter(&mut self, mut reporter: impl Reporter + 'static) {
        reporter.inherit_streams(self.stderr(), self.stdout());
        reporter.inherit_theme(self.theme());

        self.reporter = Arc::new(Box::new(reporter));
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}
