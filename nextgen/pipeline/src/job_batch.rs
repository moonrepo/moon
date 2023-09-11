use crate::job::Job;
use crate::pipe::*;
use async_trait::async_trait;
use tokio::task::JoinHandle;
use tracing::debug;

pub struct JobBatch {
    id: String,
    jobs: Vec<Job>,
}

impl JobBatch {
    pub fn new(id: String) -> Self {
        Self { id, jobs: vec![] }
    }

    pub fn add_job(&mut self, mut job: Job) -> &mut Self {
        job.batch_id = Some(self.id.clone());

        self.jobs.push(job);
        self
    }
}

#[async_trait]
impl Pipe for JobBatch {
    fn get_id(&self) -> &str {
        &self.id
    }

    async fn run(self: Box<Self>, handle: PipeHandle) -> JoinHandle<()> {
        debug!(batch = &self.id, "Running job batch");

        let mut batch = vec![];

        for job in self.jobs {
            batch.push(Box::new(job).run(handle.clone()).await)
        }

        // TODO: abort

        handle
            .run(async move {
                for job in batch {
                    job.await.unwrap();
                }
            })
            .await
    }
}
