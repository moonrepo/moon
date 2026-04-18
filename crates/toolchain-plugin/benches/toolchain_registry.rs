use criterion::{Criterion, criterion_group, criterion_main};
use moon_test_utils2::WorkspaceMocker;
use starbase_sandbox::create_empty_sandbox;
use tokio::runtime::Runtime;

fn id(label: &str) -> String {
    format!("ToolchainRegistry / {label}")
}

fn load_all(c: &mut Criterion) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_all_toolchains();

    c.bench_function(&id("load all"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            mocker.mock_toolchain_registry().load_all().await.unwrap();
        })
    });
}

fn load_many(c: &mut Criterion) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_all_toolchains();

    c.bench_function(&id("load many"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            mocker
                .mock_toolchain_registry()
                .load_many(["bun", "node", "rust"])
                .await
                .unwrap();
        })
    });
}

fn load_one(c: &mut Criterion) {
    let sandbox = create_empty_sandbox();
    let mocker = WorkspaceMocker::new(sandbox.path()).with_all_toolchains();

    c.bench_function(&id("load one"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            mocker
                .mock_toolchain_registry()
                .load("javascript")
                .await
                .unwrap();
        })
    });
}

criterion_group!(benches, load_all, load_many, load_one);
criterion_main!(benches);
