use moon_pipeline::{Job, JobBatch, Pipeline};
use starbase::App;
use std::time::Duration;
use tokio::time::sleep;

fn create_batch(id: String) -> JobBatch {
    let mut batch = JobBatch::new(id.clone());

    for i in 1..=10 {
        let job_id = format!("{id}{i}");

        batch.add_job(Job::new(job_id.clone(), async move {
            sleep(Duration::from_secs(i)).await;
            println!("{}", job_id);
        }));
    }

    batch
}

#[tokio::main]
async fn main() {
    App::setup_diagnostics();
    App::setup_tracing();

    let mut pipeline = Pipeline::default();

    // pipeline.pipe(create_batch("a".into()));

    pipeline.pipe(Job::new("a".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("a");
    }));

    pipeline.pipe(Job::new("b".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("b");
    }));

    // pipeline.pipe(create_batch("c".into()));

    pipeline.pipe(Job::new("c".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("c");
    }));

    pipeline.pipe(Job::new("d".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("d");
    }));

    pipeline.pipe(Job::new("e".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("e");
    }));

    pipeline.run().await;
}
