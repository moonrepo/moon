use assert_fs::prelude::*;
use moon_cache::{CacheEngine, ProjectsState, RunTargetState, ToolState};
use moon_utils::time;
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

        CacheEngine::load(dir.path()).await.unwrap();

        assert!(dir.path().join(".moon/cache").exists());
        assert!(dir.path().join(".moon/cache/hashes").exists());
        assert!(dir.path().join(".moon/cache/outputs").exists());
        assert!(dir.path().join(".moon/cache/states").exists());

        dir.close().unwrap();
    }
}

mod create_runfile {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn creates_runfile_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let runfile = cache
            .create_runfile("123", &"content".to_owned())
            .await
            .unwrap();

        assert!(runfile.path.exists());

        assert_eq!(
            fs::read_to_string(dir.path().join(".moon/cache/states/123/runfile.json")).unwrap(),
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
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache.cache_run_target_state("foo:bar").await.unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache.cache_run_target_state("foo:bar").await.unwrap();

        assert_eq!(
            item,
            RunTargetState {
                exit_code: 123,
                target: String::from("foo:bar"),
                path: dir.path().join(".moon/cache/states/foo/bar/lastRun.json"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = run_with_env("read", || cache.cache_run_target_state("foo:bar"))
            .await
            .unwrap();

        assert_eq!(
            item,
            RunTargetState {
                exit_code: 123,
                target: String::from("foo:bar"),
                path: dir.path().join(".moon/cache/states/foo/bar/lastRun.json"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_run_target_state("foo:bar"))
            .await
            .unwrap();

        assert_eq!(
            item,
            RunTargetState {
                target: String::from("foo:bar"),
                path: dir.path().join(".moon/cache/states/foo/bar/lastRun.json"),
                ..RunTargetState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.exit_code = 123;

        run_with_env("", || item.save()).await.unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"exitCode":123,"hash":"","lastRunTime":0,"target":"foo:bar"}"#
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_save_if_cache_off() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.exit_code = 123;

        run_with_env("off", || item.save()).await.unwrap();

        assert!(!item.path.exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_save_if_cache_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").await.unwrap();

        item.exit_code = 123;

        run_with_env("read", || item.save()).await.unwrap();

        assert!(!item.path.exists());

        dir.close().unwrap();
    }
}

mod cache_tool_state {
    use super::*;
    use moon_platform::Runtime;

    #[tokio::test]
    #[serial]
    async fn creates_parent_dir_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache
            .cache_tool_state(&Runtime::Node("1.2.3".into()))
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

        dir.child(".moon/cache/states/toolNode-1.2.3.json")
            .write_str(r#"{"lastVersionCheckTime":123}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache
            .cache_tool_state(&Runtime::Node("1.2.3".into()))
            .await
            .unwrap();

        assert_eq!(
            item,
            ToolState {
                last_version_check_time: 123,
                path: dir.path().join(".moon/cache/states/toolNode-1.2.3.json")
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/toolNode-4.5.6.json")
            .write_str(r#"{"lastVersionCheckTime":123}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let runtime = Runtime::Node("4.5.6".into());
        let item = run_with_env("read", || cache.cache_tool_state(&runtime))
            .await
            .unwrap();

        assert_eq!(
            item,
            ToolState {
                last_version_check_time: 123,
                path: dir.path().join(".moon/cache/states/toolNode-4.5.6.json")
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/toolSystem-latest.json")
            .write_str(r#"{"lastVersionCheckTime":123}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_tool_state(&Runtime::System))
            .await
            .unwrap();

        assert_eq!(
            item,
            ToolState {
                path: dir.path().join(".moon/cache/states/toolSystem-latest.json"),
                ..ToolState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let mut item = cache
            .cache_tool_state(&Runtime::Node("7.8.9".into()))
            .await
            .unwrap();

        item.last_version_check_time = 123;

        run_with_env("", || item.save()).await.unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"lastVersionCheckTime":123}"#
        );

        dir.close().unwrap();
    }
}

mod cache_projects_state {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use moon_utils::string_vec;
    use rustc_hash::FxHashMap;
    use std::time::SystemTime;

    #[tokio::test]
    #[serial]
    async fn creates_parent_dir_on_call() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/projects.json")
            .write_str(r#"{"globs":["**/*"],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert_eq!(
            item,
            ProjectsState {
                globs: string_vec!["**/*"],
                projects: FxHashMap::from([("foo".to_owned(), "bar".to_owned())]),
                path: dir.path().join(".moon/cache/states/projects.json")
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/projects.json")
            .write_str(r#"{"globs":["**/*"],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = run_with_env("read", || cache.cache_projects_state())
            .await
            .unwrap();

        assert_eq!(
            item,
            ProjectsState {
                globs: string_vec!["**/*"],
                projects: FxHashMap::from([("foo".to_owned(), "bar".to_owned())]),
                path: dir.path().join(".moon/cache/states/projects.json")
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/projects.json")
            .write_str(r#"{"globs":[],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = run_with_env("off", || cache.cache_projects_state())
            .await
            .unwrap();

        assert_eq!(
            item,
            ProjectsState {
                path: dir.path().join(".moon/cache/states/projects.json"),
                ..ProjectsState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_load_if_it_exists_but_cache_is_stale() {
        let dir = assert_fs::TempDir::new().unwrap();

        dir.child(".moon/cache/states/projects.json")
            .write_str(r#"{"globs":[],"projects":{"foo":"bar"}}"#)
            .unwrap();

        let now = time::to_millis(SystemTime::now()) - 100000;

        set_file_mtime(
            dir.path().join(".moon/cache/states/projects.json"),
            FileTime::from_unix_time((now / 1000) as i64, 0),
        )
        .unwrap();

        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let item = cache.cache_projects_state().await.unwrap();

        assert_eq!(
            item,
            ProjectsState {
                path: dir.path().join(".moon/cache/states/projects.json"),
                ..ProjectsState::default()
            }
        );

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn saves_to_cache() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let mut item = cache.cache_projects_state().await.unwrap();

        item.projects.insert("foo".to_owned(), "bar".to_owned());

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
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let hasher = TestHasher::default();

        cache.create_hash_manifest("abc123", &hasher).await.unwrap();

        assert!(cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn doesnt_create_if_cache_off() {
        let dir = assert_fs::TempDir::new().unwrap();
        let cache = CacheEngine::load(dir.path()).await.unwrap();
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
        let cache = CacheEngine::load(dir.path()).await.unwrap();
        let hasher = TestHasher::default();

        run_with_env("read", || cache.create_hash_manifest("abc123", &hasher))
            .await
            .unwrap();

        assert!(!cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }
}
