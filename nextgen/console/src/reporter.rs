use crate::buffer::ConsoleBuffer;
use crate::console::ConsoleTheme;
use miette::Error as Report;
use moon_action::{Action, ActionNode, Attempt};
use moon_config::TaskOutputStyle;
use moon_target::Target;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct PipelineReportState {
    pub duration: Option<Duration>,
}

#[derive(Default)]
pub struct TaskReportState {
    pub attempt_current: u8,
    pub attempt_total: u8,
    pub hash: Option<String>,
    pub output_streamed: bool,
    pub output_style: Option<TaskOutputStyle>,
}

pub trait Reporter: Send + Sync {
    fn inherit_streams(&mut self, _err: Arc<ConsoleBuffer>, _out: Arc<ConsoleBuffer>) {}

    fn inherit_theme(&mut self, _theme: Arc<ConsoleTheme>) {}

    fn on_pipeline_started(&self, _nodes: &[&ActionNode]) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_completed(
        &self,
        _actions: &[Action],
        _state: &PipelineReportState,
        _error: Option<&Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_aborted(
        &self,
        _actions: &[Action],
        _state: &PipelineReportState,
        _error: Option<&Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_action_started(&self, _action: &Action) -> miette::Result<()> {
        Ok(())
    }

    fn on_action_completed(&self, _action: &Action, _error: Option<&Report>) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_started(
        &self,
        _target: &Target,
        _attempt: &Attempt,
        _state: &TaskReportState,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_running(&self, _target: &Target, _secs: u32) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_finished(
        &self,
        _target: &Target,
        _attempt: &Attempt,
        _state: &TaskReportState,
        _error: Option<&Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_completed(
        &self,
        _target: &Target,
        _attempts: &[Attempt],
        _state: &TaskReportState,
        _error: Option<&Report>,
    ) -> miette::Result<()> {
        Ok(())
    }
}

pub type BoxedReporter = Box<dyn Reporter>;

pub struct EmptyReporter;

impl Reporter for EmptyReporter {}
