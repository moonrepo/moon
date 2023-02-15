use criterion::{black_box, criterion_group, criterion_main, Criterion};
use moon_action_context::ActionContext;
use moon_emitter::{Emitter, Event, EventFlow, Subscriber};
use moon_error::MoonError;
use moon_test_utils::get_fixtures_path;
use moon_workspace::Workspace;
use std::sync::Arc;
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
    let workspace_root = get_fixtures_path("cases");
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("emitter_emit", |b| {
        b.to_async(&runtime).iter(|| async {
            let workspace = Workspace::load_from(&workspace_root).unwrap();
            let emitter = Emitter::new(Arc::new(RwLock::new(workspace)));
            let context = ActionContext::default();

            emitter
                .emit(Event::PipelineStarted {
                    actions_count: 1,
                    context: &context,
                })
                .await
                .unwrap();

            emitter
                .emit(Event::PipelineStarted {
                    actions_count: 1,
                    context: &context,
                })
                .await
                .unwrap();

            emitter
                .emit(Event::PipelineStarted {
                    actions_count: 1,
                    context: &context,
                })
                .await
                .unwrap();

            black_box(emitter);
        })
    });
}

criterion_group!(emitter, emit_benchmark);
criterion_main!(emitter);
