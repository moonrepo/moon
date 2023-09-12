use async_trait::async_trait;
use moon_pipeline::*;
use starbase_events::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

fn create_job(id: &str) -> Job<RunState> {
    create_job_with_sleep(id, 500)
}

fn create_job_with_sleep(id: &str, duration: u64) -> Job<RunState> {
    Job::new(id.into(), move || async move {
        sleep(Duration::from_millis(duration)).await;
        Ok(RunState::Passed)
    })
}

fn create_isolated(job: Job<RunState>) -> IsolatedStep<RunState> {
    IsolatedStep::from(job)
}

fn create_isolated_job(id: &str) -> IsolatedStep<RunState> {
    create_isolated(create_job(id))
}

mod pipeline {
    use super::*;

    #[tokio::test]
    async fn runs_steps_in_serial() {
        let mut pipeline = Pipeline::<RunState>::new();
        pipeline.add_step(create_isolated_job("1"));
        pipeline.add_step(create_isolated_job("2"));
        pipeline.add_step(create_isolated_job("3"));

        let results = pipeline.run().await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "1");
        assert_eq!(results[1].id, "2");
        assert_eq!(results[2].id, "3");
    }

    #[tokio::test]
    async fn can_cancel_all_jobs() {
        let mut pipeline = Pipeline::<RunState>::new();
        pipeline.add_step(create_isolated_job("1"));
        pipeline.add_step(create_isolated_job("2"));
        pipeline.add_step(create_isolated_job("3"));

        let results = pipeline
            .run_with_context(|ctx| {
                ctx.cancel_token.cancel();
            })
            .await
            .unwrap();

        // Others never run since it was cancelled immediately
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].state, RunState::Cancelled);
    }

    #[tokio::test]
    async fn can_timeout_jobs() {
        let mut pipeline = Pipeline::<RunState>::new();
        pipeline.add_step(create_isolated_job("1"));

        let mut job = create_job_with_sleep("2", 2000); // 2 secs
        job.timeout = Some(1);

        pipeline.add_step(create_isolated(job));
        pipeline.add_step(create_isolated_job("3"));

        let results = pipeline.run().await.unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].state, RunState::Passed);
        assert_eq!(results[1].state, RunState::TimedOut);
        assert_eq!(results[2].state, RunState::Passed);
    }

    mod progress_event {
        use super::*;

        #[derive(Clone)]
        struct ProgressSubscriber {
            elapsed: Arc<RwLock<u32>>,
        }

        #[async_trait]
        impl Subscriber<JobProgressEvent> for ProgressSubscriber {
            fn is_once(&self) -> bool {
                false
            }

            async fn on_emit(
                &mut self,
                event: Arc<JobProgressEvent>,
                _data: Arc<RwLock<<JobProgressEvent as Event>::Data>>,
            ) -> EventResult {
                let mut value = self.elapsed.write().await;
                *value = event.elapsed;

                Ok(EventState::Continue)
            }
        }

        #[tokio::test]
        async fn tracks_progress_of_jobs() {
            let mut pipeline = Pipeline::<RunState>::new();
            let subscriber = ProgressSubscriber {
                elapsed: Arc::new(RwLock::new(0)),
            };

            pipeline.on_job_progress.subscribe(subscriber.clone()).await;

            let mut job = create_job_with_sleep("1", 3100); // 3 secs
            job.interval = Some(1);

            pipeline.add_step(create_isolated(job));
            pipeline.run().await.unwrap();

            assert_eq!(*subscriber.elapsed.read().await, 3);
        }
    }

    mod state_change_event {
        use super::*;

        #[derive(Clone)]
        struct ChangeSubscriber {
            changes: Arc<RwLock<Vec<(RunState, RunState)>>>,
        }

        #[async_trait]
        impl Subscriber<JobStateChangeEvent> for ChangeSubscriber {
            fn is_once(&self) -> bool {
                false
            }

            async fn on_emit(
                &mut self,
                event: Arc<JobStateChangeEvent>,
                _data: Arc<RwLock<<JobStateChangeEvent as Event>::Data>>,
            ) -> EventResult {
                let mut value = self.changes.write().await;
                value.push((event.prev_state, event.state));

                Ok(EventState::Continue)
            }
        }

        #[tokio::test]
        async fn receives_change_events() {
            let mut pipeline = Pipeline::<RunState>::new();
            let subscriber = ChangeSubscriber {
                changes: Arc::new(RwLock::new(vec![])),
            };

            pipeline
                .on_job_state_change
                .subscribe(subscriber.clone())
                .await;

            pipeline.add_step(create_isolated_job("1"));
            pipeline.run().await.unwrap();

            assert_eq!(
                *subscriber.changes.read().await,
                vec![
                    (RunState::Pending, RunState::Running),
                    (RunState::Running, RunState::Passed),
                ]
            );
        }

        #[tokio::test]
        async fn receives_failure_event_when_action_fails() {
            let mut pipeline = Pipeline::<RunState>::new();
            let subscriber = ChangeSubscriber {
                changes: Arc::new(RwLock::new(vec![])),
            };

            pipeline
                .on_job_state_change
                .subscribe(subscriber.clone())
                .await;

            pipeline.add_step(create_isolated(Job::new("1".into(), || async {
                Err(miette::miette!("oops"))
            })));

            let results = pipeline.run().await.unwrap();

            assert_eq!(
                *subscriber.changes.read().await,
                vec![
                    (RunState::Pending, RunState::Running),
                    (RunState::Running, RunState::Failed),
                ]
            );

            assert_eq!(results[0].error, Some("oops".to_owned()))
        }
    }
}
