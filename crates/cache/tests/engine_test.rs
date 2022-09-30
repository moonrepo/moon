use assert_fs::prelude::*;
use moon_cache::{to_millis, CacheEngine, ProjectsState, RunTargetState, ToolState};
use serde::Serialize;
use serial_test::serial;
use std::env;
use std::fs;

async fn run_with_env<T, F, Fut>(env: &str, callback: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    if env.is_empty() {
        env::remove_var("MOON_CACHE");
    } else {
        env::set_var("MOON_CACHE", env);
    }

    let result = callback().await;

    env::remove_var("MOON_CACHE");

    result
}

mod create {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn creates_dirs() {
        let dir = assert_fs::TempDir::new().unwrap();

        CacheEngine::create(dir.path()).await.unwrap();

        assert!(dir.path().join(".moon/cache").exists());
        assert!(dir.path().join(".moon/cache/hashes").exists());
        assert!(dir.path().join(".moon/cache/runs").exists());
        assert!(dir.path().join(".moon/cache/out").exists());
        assert!(dir.path().join(".moon/cache/tools").exists());

        dir.close().unwrap();
    }
}

mod delete_hash {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn deletes_files() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();

        dir.child(".moon/cache/hashes/abc123.json")
            .write_str("{}")
            .unwrap();

        dir.child(".moon/cache/out/abc123.tar.gz")
            .write_str("")
            .unwrap();

        let hash_file = cache.hashes_dir.join("abc123.json");
        let out_file = cache.outputs_dir.join("abc123.tar.gz");

        assert!(hash_file.exists());
        assert!(out_file.exists());

        cache.delete_hash("abc123").await.unwrap();

        assert!(!hash_file.exists());
        assert!(!out_file.exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_delete_if_cache_off() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();

        dir.child(".moon/cache/hashes/abc123.json")
            .write_str("{}")
            .unwrap();

        dir.child(".moon/cache/out/abc123.tar.gz")
            .write_str("")
            .unwrap();

        let hash_file = cache.hashes_dir.join("abc123.json");
        let out_file = cache.outputs_dir.join("abc123.tar.gz");

        assert!(hash_file.exists());
        assert!(out_file.exists());

        run_with_env("off", || cache.delete_hash("abc123"))
            .await
            .unwrap();

        assert!(hash_file.exists());
        assert!(out_file.exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_delete_if_cache_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();

        dir.child(".moon/cache/hashes/abc123.json")
            .write_str("{}")
            .unwrap();

        dir.child(".moon/cache/out/abc123.tar.gz")
            .write_str("")
            .unwrap();

        let hash_file = cache.hashes_dir.join("abc123.json");
        let out_file = cache.outputs_dir.join("abc123.tar.gz");

        assert!(hash_file.exists());
        assert!(out_file.exists());

        run_with_env("read", || cache.delete_hash("abc123"))
            .await
            .unwrap();

        assert!(hash_file.exists());
        assert!(out_file.exists());

        dir.close().unwrap();
    }
}

mod create_runfile {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn creates_runfile_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let runfile = cache
            .create_runfile("123", &"content".to_owned())
            .await
            .unwrap();

        assert!(runfile.path.exists());

        assert_eq!(
            fs::read_to_string(dir.path().join(".moon/cache/runs/123/runfile.json")).unwrap(),
            "\"content\""
        );

        dir.close().unwrap();
    }
}

mod cache_run_target_state {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn creates_parent_dir_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache.cache_run_target_state("foo:bar").await.unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/runs/foo/bar/lastRunState.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache.cache_run_target_state("foo:bar").await.unwrap();

        assert_eq!(
            item.item,
            RunTargetState {
                exit_code: 123,
                target: String::from("foo:bar"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/runs/foo/bar/lastRunState.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = run_with_env("read", || cache.cache_run_target_state("foo:bar"))
            .await
            .unwrap();

        assert_eq!(
            item.item,
            RunTargetState {
                exit_code: 123,
                target: String::from("foo:bar"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/runs/foo/bar/lastRunState.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_run_target_state("foo:bar"))
            .await
            .unwrap();

        assert_eq!(
            item.item,
            RunTargetState {
                target: String::from("foo:bar"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.item.exit_code = 123;

        run_with_env("", || item.save()).await.unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_save_if_cache_off() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.item.exit_code = 123;

        run_with_env("off", || item.save()).await.unwrap();

        assert!(!item.path.exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_save_if_cache_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.item.exit_code = 123;

        run_with_env("read", || item.save()).await.unwrap();

        assert!(!item.path.exists());

        dir.close().unwrap();
    }
}

mod cache_tool_state {
    use super::*;
    use moon_contract::SupportedPlatform;

    #[tokio::test]
    #[serial]
    async fn creates_parent_dir_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache
            .cache_tool_state(&SupportedPlatform::Node("1.2.3".into()))
            .await
            .unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/tools/node-v1.2.3.json")
            .write_str(r#"{"lastDepsInstallTime":123}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache
            .cache_tool_state(&SupportedPlatform::Node("1.2.3".into()))
            .await
            .unwrap();

        assert_eq!(
            item.item,
            ToolState {
                last_deps_install_time: 123,
                last_version_check_time: 0,
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/tools/node-v4.5.6.json")
            .write_str(r#"{"lastDepsInstallTime":123}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let platform = SupportedPlatform::Node("4.5.6".into());
        let item = run_with_env("read", || cache.cache_tool_state(&platform))
            .await
            .unwrap();

        assert_eq!(
            item.item,
            ToolState {
                last_deps_install_time: 123,
                last_version_check_time: 0,
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/tools/system.json")
            .write_str(r#"{"lastDepsInstallTime":123}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_tool_state(&SupportedPlatform::System))
            .await
            .unwrap();

        assert_eq!(item.item, ToolState::default());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let mut item = cache
            .cache_tool_state(&SupportedPlatform::Node("7.8.9".into()))
            .await
            .unwrap();

        item.item.last_deps_install_time = 123;

        run_with_env("", || item.save()).await.unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"lastDepsInstallTime":123,"lastVersionCheckTime":0}"#
        );

        dir.close().unwrap();
    }
}

mod cache_projects_state {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use moon_utils::string_vec;
    use std::collections::HashMap;
    use std::time::SystemTime;

    #[tokio::test]
    #[serial]
    async fn creates_parent_dir_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/projectsState.json")
            .write_str(r#"{"globs":["**/*"],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert_eq!(
            item.item,
            ProjectsState {
                globs: string_vec!["**/*"],
                projects: HashMap::from([("foo".to_owned(), "bar".to_owned())]),
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/projectsState.json")
            .write_str(r#"{"globs":["**/*"],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = run_with_env("read", || cache.cache_projects_state())
            .await
            .unwrap();

        assert_eq!(
            item.item,
            ProjectsState {
                globs: string_vec!["**/*"],
                projects: HashMap::from([("foo".to_owned(), "bar".to_owned())]),
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/projectsState.json")
            .write_str(r#"{"globs":[],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_projects_state())
            .await
            .unwrap();

        assert_eq!(item.item, ProjectsState::default());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_stale() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/projectsState.json")
            .write_str(r#"{"globs":[],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let now = to_millis(SystemTime::now()) - 100000;

        set_file_mtime(
            dir.path().join(".moon/cache/projectsState.json"),
            FileTime::from_unix_time((now / 1000) as i64, 0),
        )
        .unwrap();

        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert_eq!(item.item, ProjectsState::default());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let mut item = cache.cache_projects_state().await.unwrap();

        item.item
            .projects
            .insert("foo".to_owned(), "bar".to_owned());

        run_with_env("", || item.save()).await.unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"globs":[],"projects":{"foo":"bar"}}"#
        );

        dir.close().unwrap();
    }
}

mod create_hash_manifest {
    use super::*;
    use serde::Deserialize;

    #[derive(Default, Deserialize, Serialize)]
    struct TestHasher {
        field: String,
    }

    #[tokio::test]
    #[serial]
    async fn creates_hash_file() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let hasher = TestHasher::default();

        cache.create_hash_manifest("abc123", &hasher).await.unwrap();

        assert!(cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_create_if_cache_off() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let hasher = TestHasher::default();

        run_with_env("off", || cache.create_hash_manifest("abc123", &hasher))
            .await
            .unwrap();

        assert!(!cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_create_if_cache_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::create(dir.path()).await.unwrap();
        let hasher = TestHasher::default();

        run_with_env("read", || cache.create_hash_manifest("abc123", &hasher))
            .await
            .unwrap();

        assert!(!cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }
}
