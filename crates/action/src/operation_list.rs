use crate::OperationMeta;
use crate::{action::ActionStatus, operation::*};
use serde::{Deserialize, Serialize};
use std::mem;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct OperationList(Vec<Operation>);

impl OperationList {
    pub fn get_final_status(&self) -> ActionStatus {
        self.get_last_process()
            .map(|op| op.status)
            .unwrap_or(ActionStatus::Invalid)
    }

    pub fn get_hash(&self) -> Option<&str> {
        self.0
            .iter()
            .find(|op| op.meta.is_hash_generation())
            .and_then(|op| match &op.meta {
                OperationMeta::HashGeneration(inner) => inner.hash.as_deref(),
                _ => None,
            })
    }

    /// Returns the last "metric based" operation.
    pub fn get_last_metric(&self) -> Option<&Operation> {
        self.0.iter().rfind(|op| {
            op.meta.is_archive_creation()
                || op.meta.is_hash_generation()
                || op.meta.is_mutex_acquisition()
        })
    }

    /// Returns the last "process based" operation.
    pub fn get_last_process(&self) -> Option<&Operation> {
        self.0.iter().rfind(|op| {
            op.meta.is_no_operation()
                || op.meta.is_output_hydration()
                || op.meta.is_process_execution()
                || op.meta.is_sync_operation()
                || op.meta.is_task_execution()
        })
    }

    /// Returns the last task execution operation.
    pub fn get_last_execution(&self) -> Option<&Operation> {
        self.0.iter().rfind(|op| op.meta.is_task_execution())
    }

    pub fn is_flaky(&self) -> bool {
        let mut attempt_count = 0;
        let mut any_failed = false;
        let mut last_passed = false;

        for operation in &self.0 {
            if operation.meta.is_task_execution() {
                attempt_count += 1;
                last_passed = operation.has_passed();

                if operation.has_failed() {
                    any_failed = true;
                }
            }
        }

        attempt_count > 0 && any_failed && last_passed
    }

    pub fn merge(&mut self, other: OperationList) {
        self.0.extend(other.0);
    }

    pub fn take(&mut self) -> Self {
        Self(mem::take(&mut self.0))
    }
}

impl Deref for OperationList {
    type Target = Vec<Operation>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OperationList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
