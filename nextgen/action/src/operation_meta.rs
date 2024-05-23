use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct OperationMetaHash {
    pub hash: Option<String>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationMetaOutput {
    pub command: Option<String>,
    pub exit_code: Option<i32>,
    pub stderr: Option<Arc<String>>,
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
    OutputHydration,
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
        matches!(self, Self::OutputHydration)
    }

    pub fn is_task_execution(&self) -> bool {
        matches!(self, Self::TaskExecution(_))
    }
}
