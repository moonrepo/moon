use moon_task::{Target, Task};
use std::sync::Arc;

pub trait TaskLookup {
    fn get_task(&self, target: &Target) -> miette::Result<Arc<Task>>;
}
