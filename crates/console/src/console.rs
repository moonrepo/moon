use crate::buffer::*;
use crate::prompts::create_theme;
use crate::reporter::*;
use inquire::ui::RenderConfig;
use moon_common::is_formatted_output;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use tracing::trace;

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
        trace!("Creating buffered console");

        // TODO
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
}
