use crate::buffer::ConsoleBuffer;
use crate::console::ConsoleTheme;
use std::sync::Arc;

pub trait Reporter: Send + Sync {
    fn inherit_streams(&mut self, _err: Arc<ConsoleBuffer>, _out: Arc<ConsoleBuffer>) {}

    fn inherit_theme(&mut self, _theme: Arc<ConsoleTheme>) {}
}

pub type BoxedReporter = Box<dyn Reporter>;

pub struct EmptyReporter;

impl Reporter for EmptyReporter {}
