use crate::action::ActionStatus;
use crate::operation_meta::*;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::process::Output;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<Duration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<NaiveDateTime>,

    pub meta: OperationMeta,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<Operation>,

    pub started_at: NaiveDateTime,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Operation {
    pub fn new(meta: OperationMeta) -> Self {
        Operation {
            duration: None,
            finished_at: None,
            meta,
            operations: vec![],
            started_at: now_timestamp(),
            start_time: Some(Instant::now()),
            status: ActionStatus::Running,
        }
    }

    pub fn new_finished(meta: OperationMeta, status: ActionStatus) -> Self {
        let time = now_timestamp();

        Operation {
            duration: None,
            finished_at: Some(time),
            meta,
            operations: vec![],
            started_at: time,
            start_time: None,
            status,
        }
    }

    pub fn get_exec_output(&self) -> Option<&OperationMetaOutput> {
        match &self.meta {
            OperationMeta::OutputHydration(output)
            | OperationMeta::ProcessExecution(output)
            | OperationMeta::TaskExecution(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_exec_output_mut(&mut self) -> Option<&mut OperationMetaOutput> {
        match &mut self.meta {
            OperationMeta::OutputHydration(output)
            | OperationMeta::ProcessExecution(output)
            | OperationMeta::TaskExecution(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_exec_output_status(&self) -> String {
        self.get_exec_output()
            .and_then(|output| {
                if let Some(code) = output.exit_code {
                    return Some(format!("exit code {code}"));
                }

                if let Some(status) = output.exit_status {
                    return Some(status.to_string());
                }

                None
            })
            .unwrap_or_else(|| match self.status {
                ActionStatus::Skipped => "skipped".into(),
                ActionStatus::TimedOut => "timed out".into(),
                _ => "unknown failure".into(),
            })
    }

    pub fn get_sync_result(&self) -> Option<&OperationMetaSync> {
        match &self.meta {
            OperationMeta::SyncOperation(output) => Some(output),
            _ => None,
        }
    }

    pub fn get_sync_result_mut(&mut self) -> Option<&mut OperationMetaSync> {
        match &mut self.meta {
            OperationMeta::SyncOperation(output) => Some(output),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match &self.meta {
            OperationMeta::NoOperation => "NoOperation",
            OperationMeta::OutputHydration(_) => "OutputHydration",
            OperationMeta::ProcessExecution(_) => "ProcessExecution",
            OperationMeta::SyncOperation(_) => "SyncOperation",
            OperationMeta::TaskExecution(_) => "TaskExecution",
            OperationMeta::ArchiveCreation => "ArchiveCreation",
            OperationMeta::HashGeneration(_) => "HashGeneration",
            OperationMeta::MutexAcquisition => "MutexAcquisition",
        }
    }

    pub fn finish(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn finish_from_output(&mut self, process_output: Output) {
        if let Some(output) = self.get_exec_output_mut() {
            output.exit_code = process_output.status.code();
            output.exit_status = Some(process_output.status);
            output.set_stderr(String::from_utf8(process_output.stderr).unwrap_or_default());
            output.set_stdout(String::from_utf8(process_output.stdout).unwrap_or_default());
        }

        self.finish(if process_output.status.success() {
            ActionStatus::Passed
        } else {
            ActionStatus::Failed
        });
    }

    pub fn has_failed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Aborted | ActionStatus::Failed | ActionStatus::TimedOut
        )
    }

    pub fn has_passed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote | ActionStatus::Passed
        )
    }

    pub fn has_output(&self) -> bool {
        self.get_exec_output().is_some_and(|output| {
            output.stderr.as_ref().is_some_and(|err| !err.is_empty())
                || output.stdout.as_ref().is_some_and(|out| !out.is_empty())
        })
    }

    pub fn is_cached(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote
        )
    }

    pub fn track<T, F>(mut self, func: F) -> miette::Result<Self>
    where
        F: FnOnce() -> miette::Result<T>,
    {
        Self::do_track(&mut self, func).map(|_| self)
    }

    pub fn track_with_check<T, F, C>(mut self, func: F, checker: C) -> miette::Result<Self>
    where
        F: FnOnce() -> miette::Result<T>,
        C: FnOnce(T) -> bool,
    {
        Self::do_track_with_check(&mut self, func, checker).map(|_| self)
    }

    pub async fn track_async<T, F, Fut>(mut self, func: F) -> miette::Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<T>>,
    {
        Self::do_track_async(&mut self, func).await.map(|_| self)
    }

    pub async fn track_async_changed<F, Fut>(mut self, func: F) -> miette::Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<Vec<PathBuf>>>,
    {
        let result = func().await;

        if let Ok(files) = &result {
            if let Some(sync) = self.get_sync_result_mut() {
                sync.changed_files.extend(files.clone());
            }
        }

        self.handle_track(result, |_| true).map(|_| self)
    }

    pub async fn track_async_with_check<T, F, Fut, C>(
        mut self,
        func: F,
        checker: C,
    ) -> miette::Result<Self>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<T>>,
        C: FnOnce(T) -> bool,
    {
        Self::do_track_async_with_check(&mut self, func, checker)
            .await
            .map(|_| self)
    }

    pub(crate) fn handle_track<T>(
        &mut self,
        result: miette::Result<T>,
        checker: impl FnOnce(T) -> bool,
    ) -> miette::Result<()> {
        match result {
            Ok(value) => {
                self.finish(if checker(value) {
                    ActionStatus::Passed
                } else {
                    ActionStatus::Skipped
                });

                Ok(())
            }
            Err(error) => {
                self.finish(ActionStatus::Failed);

                Err(error)
            }
        }
    }

    // Constructors

    pub fn archive_creation() -> Self {
        Self::new(OperationMeta::ArchiveCreation)
    }

    pub fn hash_generation() -> Self {
        Self::new(OperationMeta::HashGeneration(Default::default()))
    }

    pub fn no_operation() -> Self {
        Self::new(OperationMeta::NoOperation)
    }

    pub fn mutex_acquisition() -> Self {
        Self::new(OperationMeta::MutexAcquisition)
    }

    pub fn output_hydration() -> Self {
        Self::new(OperationMeta::OutputHydration(Default::default()))
    }

    pub fn process_execution(command: impl AsRef<str>) -> Self {
        Self::new(OperationMeta::ProcessExecution(Box::new(
            OperationMetaOutput {
                command: Some(command.as_ref().to_owned()),
                ..Default::default()
            },
        )))
    }

    pub fn sync_operation(label: impl AsRef<str>) -> Self {
        Self::new(OperationMeta::SyncOperation(Box::new(OperationMetaSync {
            changed_files: vec![],
            label: label.as_ref().to_owned(),
        })))
    }

    pub fn task_execution(command: impl AsRef<str>) -> Self {
        Self::new(OperationMeta::TaskExecution(Box::new(
            OperationMetaOutput {
                command: Some(command.as_ref().to_owned()),
                ..Default::default()
            },
        )))
    }

    // Trackers

    pub fn do_track<T, F>(op: &mut Operation, func: F) -> miette::Result<()>
    where
        F: FnOnce() -> miette::Result<T>,
    {
        op.handle_track(func(), |_| true)
    }

    pub fn do_track_with_check<T, F, C>(
        op: &mut Operation,
        func: F,
        checker: C,
    ) -> miette::Result<()>
    where
        F: FnOnce() -> miette::Result<T>,
        C: FnOnce(T) -> bool,
    {
        op.handle_track(func(), checker)
    }

    pub async fn do_track_async<T, F, Fut>(op: &mut Operation, func: F) -> miette::Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<T>>,
    {
        op.handle_track(func().await, |_| true)
    }

    pub async fn do_track_async_with_check<T, F, Fut, C>(
        op: &mut Operation,
        func: F,
        checker: C,
    ) -> miette::Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<T>>,
        C: FnOnce(T) -> bool,
    {
        op.handle_track(func().await, checker)
    }
}
