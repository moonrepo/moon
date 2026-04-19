use criterion::{Criterion, criterion_group, criterion_main};
use moon_bench_utils::handle_unwrap;
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::create_empty_sandbox;
use tokio::runtime::Runtime;

fn id(label: &str) -> String {
    label.to_string()
}

fn load(c: &mut Criterion) {
    let mut group = c.benchmark_group("ToolchainRegistry");
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_all_toolchains();

    group.bench_function(id("load_all"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let registry = mocker.mock_toolchain_registry();

            handle_unwrap(registry.load_all().await);
            drop(registry);
        })
    });

    group.bench_function(id("load_many"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let registry = mocker.mock_toolchain_registry();

            handle_unwrap(registry.load_many(["bun", "node", "rust"]).await);
            drop(registry);
        })
    });

    group.bench_function(id("load_one"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let registry = mocker.mock_toolchain_registry();

            handle_unwrap(registry.load("javascript").await);
            drop(registry);
        })
    });

    group.finish();
}

criterion_group!(benches, load);
criterion_main!(benches);
