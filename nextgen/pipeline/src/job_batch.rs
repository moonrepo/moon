use crate::pipe::Pipe;
use async_trait::async_trait;

pub struct JobBatch {}

#[async_trait]
impl Pipe for JobBatch {
    async fn run(&self) {}
}
