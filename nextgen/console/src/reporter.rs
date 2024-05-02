use crate::buffer::ConsoleBuffer;
use crate::console::ConsoleTheme;
use moon_action::Action;
use std::sync::Arc;

pub trait Reporter: Send + Sync {
    fn inherit_streams(
        &mut self,
        _err: Arc<ConsoleBuffer>,
        _out: Arc<ConsoleBuffer>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn inherit_theme(&mut self, _theme: Arc<ConsoleTheme>) -> miette::Result<()> {
        Ok(())
    }

    fn on_action_started(&mut self, _action: &Action) -> miette::Result<()> {
        Ok(())
    }

    fn on_action_completed(
        &mut self,
        _action: &Action,
        _error: Option<miette::Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_aborted(&mut self, _error: Option<miette::Report>) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_started(&mut self) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_completeed(&mut self, _error: Option<miette::Report>) -> miette::Result<()> {
        Ok(())
    }
}

pub type BoxedReporter = Box<dyn Reporter>;

pub struct EmptyReporter;

impl Reporter for EmptyReporter {}
