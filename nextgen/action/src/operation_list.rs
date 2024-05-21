use crate::{action::ActionStatus, operation::*};
use serde::{Deserialize, Serialize};
use std::mem;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct OperationList(Vec<Operation>);

impl OperationList {
    pub fn get_final_status(&self) -> ActionStatus {
        for operation in self.0.iter().rev() {
            // Only these types of operations should be used for the final action
            // status, as the other operation types are merely for metrics tracking
            if matches!(
                operation.type_of,
                OperationType::NoOperation
                    | OperationType::TaskExecution
                    | OperationType::OutputHydration
            ) {
                return operation.status;
            }
        }

        ActionStatus::Invalid
    }

    pub fn get_hash(&self) -> Option<&str> {
        self.0
            .iter()
            .find(|op| op.hash.is_some())
            .and_then(|op| op.hash.as_deref())
    }

    pub fn get_last_execution(&self) -> Option<&Operation> {
        self.0
            .iter()
            .rfind(|op| matches!(op.type_of, OperationType::TaskExecution))
    }

    pub fn is_flaky(&self) -> bool {
        let mut attempt_count = 0;
        let mut any_failed = false;
        let mut last_passed = false;

        for operation in &self.0 {
            if matches!(operation.type_of, OperationType::TaskExecution) {
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
