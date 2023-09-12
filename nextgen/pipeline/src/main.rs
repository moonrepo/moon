use moon_pipeline::{IsolatedStep, Job, Pipeline};
use starbase::App;
use std::time::Duration;
use tokio::time::sleep;

// fn create_batch(id: String) -> JobBatch {
//     let mut batch = JobBatch::new(id.clone());

//     for i in 1..=10 {
//         let job_id = format!("{id}{i}");

//         batch.add_job(Job::new(job_id.clone(), async move {
//             sleep(Duration::from_secs(i)).await;
//             println!("{}", job_id);
//         }));
//     }

//     batch
// }

#[derive(Debug)]
struct TestResult {}

#[tokio::main]
async fn main() {
    App::setup_diagnostics();
    App::setup_tracing();

    let mut pipeline = Pipeline::<TestResult>::new();

    // pipeline.pipe(create_batch("a".into()));

    pipeline.add_step(IsolatedStep::new("a".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("a");
        Ok(TestResult {})
    }));

    pipeline.add_step(IsolatedStep::new("b".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("b");
        Ok(TestResult {})
    }));

    // pipeline.pipe(create_batch("c".into()));

    pipeline.add_step(IsolatedStep::new("c".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("c");
        Ok(TestResult {})
    }));

    pipeline.add_step(IsolatedStep::new("d".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("d");
        Ok(TestResult {})
    }));

    pipeline.add_step(IsolatedStep::new("e".into(), async {
        sleep(Duration::from_secs(1)).await;
        println!("e");
        Ok(TestResult {})
    }));

    let results = pipeline.run().await.unwrap();

    dbg!(results);
}
