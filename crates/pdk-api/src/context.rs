use moon_common::Id;
use moon_project::ProjectFragment;
use std::time::{Duration, Instant, SystemTime};
use warpgate_api::{AnyResult, VirtualPath, api_enum, api_struct};

api_struct!(
    /// Information about the current moon workspace.
    pub struct MoonContext {
        /// Virtual path to the current working directory.
        pub working_dir: VirtualPath,

        /// Virtual path to the workspace root.
        pub workspace_root: VirtualPath,
    }
);

impl MoonContext {
    /// Return the provided file path as an absolute path (using virtual paths).
    /// If the path is already absolute (either real or virtual), return it.
    /// Otherwise prefix the path with the current working directory.
    pub fn get_absolute_path<T: AsRef<std::path::Path>>(&self, path: T) -> VirtualPath {
        let path = path.as_ref();

        if path.is_absolute() {
            return VirtualPath::Real(path.to_owned());
        }

        self.working_dir.join(path)
    }

    /// Return an absolute path for the provided project.
    pub fn get_project_root(&self, project: &ProjectFragment) -> VirtualPath {
        self.get_project_root_from_source(&project.source)
    }

    /// Return an absolute path for the provided project file path source.
    pub fn get_project_root_from_source(&self, source: &str) -> VirtualPath {
        if source.is_empty() || source == "." {
            self.workspace_root.clone()
        } else {
            self.workspace_root.join(source)
        }
    }
}

api_enum!(
    /// The status of a performed operation.
    #[derive(Default)]
    pub enum OperationStatus {
        #[default]
        Pending,
        Failed,
        Passed,
    }
);

api_struct!(
    /// An operation can be used to track timings, statuses, and results for
    /// business logic that was performed within an action (a plugin function).
    #[serde(default)]
    pub struct Operation {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub duration: Option<Duration>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub finished_at: Option<SystemTime>,

        pub id: Id,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub started_at: Option<SystemTime>,

        #[serde(skip)]
        pub start_time: Option<Instant>,

        pub status: OperationStatus,
    }
);

impl Operation {
    /// Create a new operation with a unique ID. The ID
    /// will be converted to kebab-case when serialized.
    pub fn new(id: impl AsRef<str>) -> AnyResult<Self> {
        Ok(Operation {
            duration: None,
            error: None,
            finished_at: None,
            id: Id::new(id.as_ref())?,
            started_at: Some(SystemTime::now()),
            start_time: Some(Instant::now()),
            status: OperationStatus::Pending,
        })
    }

    /// Mark the operation as finished with the provided status.
    pub fn finish(&mut self, status: OperationStatus) {
        self.finished_at = Some(SystemTime::now());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    /// Mark the operation as finished based on the state of a result.
    pub fn finish_with_result<T, E: std::fmt::Display>(&mut self, result: &Result<T, E>) {
        match result {
            Ok(_) => {
                self.finish(OperationStatus::Passed);
            }
            Err(error) => {
                self.finish(OperationStatus::Failed);
                self.error = Some(error.to_string());
            }
        }
    }

    /// Create a new operation and track its state when executing
    /// the provided callback.
    pub fn track<I, F, R>(id: I, func: F) -> AnyResult<(Self, R)>
    where
        I: AsRef<str>,
        F: FnOnce() -> AnyResult<R>,
    {
        let mut op = Self::new(id)?;

        match func() {
            Ok(res) => {
                op.finish(OperationStatus::Passed);

                Ok((op, res))
            }
            Err(error) => Err(error),
        }
    }

    /// Create a new operation and track its state when executing
    /// the provided callback, but do not bubble up a result error.
    pub fn track_with_error<I, F, R>(id: I, func: F) -> AnyResult<(Self, Option<R>)>
    where
        I: AsRef<str>,
        F: FnOnce() -> AnyResult<R>,
    {
        let mut op = Self::new(id)?;

        let res = match func() {
            Ok(res) => {
                op.finish(OperationStatus::Passed);

                (op, Some(res))
            }
            Err(error) => {
                op.finish(OperationStatus::Failed);
                op.error = Some(error.to_string());

                (op, None)
            }
        };

        Ok(res)
    }
}
