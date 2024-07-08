use crate::buffer::*;
use crate::prompts::create_theme;
use crate::reporter::*;
use inquire::ui::RenderConfig;
use moon_common::is_formatted_output;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tracing::debug;

pub type ConsoleTheme = RenderConfig<'static>;

pub struct Console {
    pub err: Arc<ConsoleBuffer>,
    err_handle: Option<JoinHandle<()>>,

    pub out: Arc<ConsoleBuffer>,
    out_handle: Option<JoinHandle<()>>,

    pub reporter: Arc<BoxedReporter>,

    quiet: Arc<AtomicBool>,
    theme: Arc<ConsoleTheme>,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        debug!("Creating buffered console");

        let quiet = Arc::new(AtomicBool::new(quiet || is_formatted_output()));

        let mut err = ConsoleBuffer::new(ConsoleStream::Stderr);
        err.quiet = Some(Arc::clone(&quiet));

        let mut out = ConsoleBuffer::new(ConsoleStream::Stdout);
        out.quiet = Some(Arc::clone(&quiet));

        Self {
            err_handle: err.handle.take(),
            err: Arc::new(err),
            out_handle: out.handle.take(),
            out: Arc::new(out),
            quiet,
            reporter: Arc::new(Box::new(EmptyReporter)),
            theme: Arc::new(create_theme()),
        }
    }

    pub fn new_testing() -> Self {
        Self {
            err: Arc::new(ConsoleBuffer::new_testing(ConsoleStream::Stderr)),
            err_handle: None,
            out: Arc::new(ConsoleBuffer::new_testing(ConsoleStream::Stdout)),
            out_handle: None,
            quiet: Arc::new(AtomicBool::new(false)),
            reporter: Arc::new(Box::new(EmptyReporter)),
            theme: Arc::new(ConsoleTheme::empty()),
        }
    }

    pub fn close(&mut self) -> miette::Result<()> {
        debug!("Closing console and flushing buffered output");

        self.err.close()?;
        self.out.close()?;

        if let Some(handle) = self.err_handle.take() {
            let _ = handle.join();
        }

        if let Some(handle) = self.out_handle.take() {
            let _ = handle.join();
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

    pub fn set_reporter(&mut self, mut reporter: impl Reporter + 'static) {
        reporter.inherit_streams(self.stderr(), self.stdout());
        reporter.inherit_theme(self.theme());

        self.reporter = Arc::new(Box::new(reporter));
    }
}

impl Clone for Console {
    fn clone(&self) -> Self {
        Self {
            err: self.err.clone(),
            err_handle: None,
            out: self.out.clone(),
            out_handle: None,
            quiet: self.quiet.clone(),
            reporter: self.reporter.clone(),
            theme: self.theme.clone(),
        }
    }
}
