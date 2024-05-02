use moon_action::Attempt;
use moon_console::Reporter;
use moon_task::Task;

pub struct TaskReportState {
    pub attempt_current: u8,
    pub attempt_total: u8,
    pub hash: Option<String>,
    pub streamed_output: bool,
}

pub trait TaskReporterExt: Reporter {
    fn on_task_started(
        &mut self,
        _task: &Task,
        _attempt: &Attempt,
        _state: &TaskReportState,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_running(&mut self, _task: &Task, _state: &TaskReportState) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_finished(
        &mut self,
        _task: &Task,
        _attempt: &Attempt,
        _state: &TaskReportState,
        _error: Option<miette::Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_completed(
        &mut self,
        _task: &Task,
        _attempts: &[Attempt],
        _state: &TaskReportState,
        _error: Option<miette::Report>,
    ) -> miette::Result<()> {
        Ok(())
    }
}
