use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use moon_emitter::{Emitter, Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_utils::test::get_fixtures_dir;
use moon_workspace::Workspace;
use tokio::sync::RwLock;

struct TestSubscriber;

#[async_trait::async_trait]
impl Subscriber for TestSubscriber {
    async fn on_emit<'e>(
        &mut self,
        _event: &Event<'e>,
        _workspace: &Workspace,
    ) -> Result<EventFlow, MoonError> {
        Ok(EventFlow::Continue)
    }
}

pub fn emit_benchmark(c: &mut Criterion) {
    let workspace_root = get_fixtures_dir("cases");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("emit", |b| {
        b.to_async(&runtime).iter(|| async {
            let workspace = Workspace::create(&workspace_root).await.unwrap();
            let emitter = Emitter::new(Arc::new(RwLock::new(workspace)));

            emitter
                .emit(Event::RunnerStarted { actions_count: 1 })
                .await
                .unwrap();

            emitter
                .emit(Event::RunnerStarted { actions_count: 1 })
                .await
                .unwrap();

            emitter
                .emit(Event::RunnerStarted { actions_count: 1 })
                .await
                .unwrap();
        })
    });
}

criterion_group!(emitter, emit_benchmark);
criterion_main!(emitter);
