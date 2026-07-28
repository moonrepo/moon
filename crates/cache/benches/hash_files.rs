use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use moon_bench_utils::handle_unwrap;
use moon_cache::{CacheContext, CacheEngine};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{CacheConfig, RemoteConfig};
use moon_vcs::Vcs;
use moon_vcs::git::Git;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_engine(sandbox: &Sandbox) -> CacheEngine {
    CacheEngine::new(CacheContext {
        cache_dir: sandbox.path().join("cache"),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir: sandbox.path().to_path_buf(),
        remote_config: Arc::new(RemoteConfig::default()),
        remote_debug: false,
        workspace_root: sandbox.path().to_path_buf(),
    })
    .unwrap()
}

fn id(max: u16, label: &str) -> BenchmarkId {
    BenchmarkId::new(label, max)
}

fn create_sandbox_with_files() -> Sandbox {
    let sandbox = create_empty_sandbox();
    sandbox.enable_git();

    for i in 0..=1000 {
        std::fs::write(sandbox.path().join(format!("file{i}.txt")), i.to_string()).unwrap();
    }

    sandbox
}

fn get_relative_file_paths(limit: usize) -> Vec<WorkspaceRelativePathBuf> {
    (0..=limit)
        .map(|i| WorkspaceRelativePathBuf::from(format!("file{i}.txt")))
        .collect()
}

fn cas(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cas");
    let sandbox = create_sandbox_with_files();

    group.bench_function(id(100, "hash_files"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            handle_unwrap(
                create_engine(&sandbox)
                    .hash_files(sandbox.path(), &get_relative_file_paths(100))
                    .await,
            );
        })
    });

    group.bench_function(id(1000, "hash_files"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            handle_unwrap(
                create_engine(&sandbox)
                    .hash_files(sandbox.path(), &get_relative_file_paths(1000))
                    .await,
            );
        })
    });

    group.finish();
}

fn vcs_git(c: &mut Criterion) {
    let mut group = c.benchmark_group("VcsGit");
    let sandbox = create_sandbox_with_files();

    group.bench_function(id(100, "get_file_hashes"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let git = Git::load(sandbox.path(), "master", &["origin".to_string()]).unwrap();

            git.get_file_hashes(&get_relative_file_paths(100), true)
                .await
                .unwrap();
        })
    });

    group.bench_function(id(1000, "get_file_hashes"), |b| {
        b.to_async(Runtime::new().unwrap()).iter(async || {
            let git = Git::load(sandbox.path(), "master", &["origin".to_string()]).unwrap();

            git.get_file_hashes(&get_relative_file_paths(1000), true)
                .await
                .unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, cas, vcs_git);
criterion_main!(benches);
