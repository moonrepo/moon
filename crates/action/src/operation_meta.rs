use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Arc;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OperationMetaHash {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OperationMetaSync {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<PathBuf>,

    pub label: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OperationMetaOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    #[serde(skip)]
    pub exit_status: Option<ExitStatus>,

    #[serde(skip)]
    pub stderr: Option<Arc<String>>,

    #[serde(skip)]
    pub stdout: Option<Arc<String>>,
}

impl OperationMetaOutput {
    pub fn get_exit_code(&self) -> i32 {
        self.exit_code.unwrap_or(-1)
    }

    pub fn set_stderr(&mut self, output: String) {
        if !output.is_empty() {
            self.stderr = Some(Arc::new(output));
        }
    }

    pub fn set_stdout(&mut self, output: String) {
        if !output.is_empty() {
            self.stdout = Some(Arc::new(output));
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OperationMeta {
    // Processes
    #[default]
    NoOperation,
    OutputHydration(Box<OperationMetaOutput>),
    ProcessExecution(Box<OperationMetaOutput>),
    SyncOperation(Box<OperationMetaSync>),
    TaskExecution(Box<OperationMetaOutput>),

    // Metrics
    ArchiveCreation,
    HashGeneration(Box<OperationMetaHash>),
    MutexAcquisition,
}

impl OperationMeta {
    pub fn is_archive_creation(&self) -> bool {
        matches!(self, Self::ArchiveCreation)
    }

    pub fn is_hash_generation(&self) -> bool {
        matches!(self, Self::HashGeneration(_))
    }

    pub fn is_no_operation(&self) -> bool {
        matches!(self, Self::NoOperation)
    }

    pub fn is_mutex_acquisition(&self) -> bool {
        matches!(self, Self::MutexAcquisition)
    }

    pub fn is_output_hydration(&self) -> bool {
        matches!(self, Self::OutputHydration(_))
    }

    pub fn is_process_execution(&self) -> bool {
        matches!(self, Self::ProcessExecution(_))
    }

    pub fn is_sync_operation(&self) -> bool {
        matches!(self, Self::SyncOperation(_))
    }

    pub fn is_task_execution(&self) -> bool {
        matches!(self, Self::TaskExecution(_))
    }

    pub fn set_hash(&mut self, hash: impl AsRef<str>) {
        if let Self::HashGeneration(inner) = self {
            inner.hash = Some(hash.as_ref().to_owned());
        }
    }
}
