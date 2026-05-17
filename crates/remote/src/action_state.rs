use crate::action_result::create_timestamp_from_naive;
use crate::blob::*;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, ExecutedActionMetadata,
};
use moon_action::Operation;
use moon_hash::Digest;
use moon_task::Task;

pub struct ActionState<'task> {
    task: &'task Task,

    // RE API
    pub action_result: Option<ActionResult>,
    pub digest: Digest,

    // Outputs to upload
    pub blobs: Vec<CompressableBlob>,

    // Bytes of our hashed manifest
    pub bytes: Vec<u8>,
}

impl ActionState<'_> {
    pub fn new(digest: Digest, task: &Task) -> ActionState<'_> {
        ActionState {
            task,
            action_result: None,
            digest,
            blobs: vec![],
            bytes: vec![],
        }
    }

    pub fn set_action_result(&mut self, result: ActionResult) {
        self.action_result = Some(result);
    }

    pub fn extract_for_upload(&mut self) -> Option<(ActionResult, Vec<CompressableBlob>)> {
        self.action_result
            .take()
            .map(|result| (result, self.blobs.drain(0..).collect::<Vec<_>>()))
    }
}
