use crate::pipe::Pipe;
use async_trait::async_trait;

pub struct Job {}

#[async_trait]
impl Pipe for Job {
    async fn run(&self) {}
}
