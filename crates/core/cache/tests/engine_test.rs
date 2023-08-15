use moon_cache::{CacheEngine, RunTargetState};
use moon_test_utils::{assert_fs::prelude::*, create_temp_dir};
use serde::Serialize;
use serial_test::serial;
use std::env;
use std::fs;

fn run_with_env<T, F>(env: &str, callback: F) -> T
where
    F: FnOnce() -> T,
{
    if env.is_empty() {
        env::remove_var("MOON_CACHE");
    } else {
        env::set_var("MOON_CACHE", env);
    }

    let result = callback();

    env::remove_var("MOON_CACHE");

    result
}

mod create {
    use super::*;

    #[test]
    #[serial]
    fn creates_dirs() {
        let dir = create_temp_dir();

        CacheEngine::load(dir.path()).unwrap();

        assert!(dir.path().join(".moon/cache").exists());
        assert!(dir.path().join(".moon/cache/hashes").exists());
        assert!(dir.path().join(".moon/cache/outputs").exists());
        assert!(dir.path().join(".moon/cache/states").exists());

        dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn creates_cachedir_tag() {
        let dir = create_temp_dir();

        CacheEngine::load(dir.path()).unwrap();

        assert!(dir.path().join(".moon/cache/CACHEDIR.TAG").exists());

        dir.close().unwrap();
    }
}

mod create_snapshot {
    use super::*;

    #[test]
    #[serial]
    fn creates_on_call() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let snapshot = cache.create_snapshot("123", &"content".to_owned()).unwrap();

        assert!(snapshot.path.exists());

        assert_eq!(
            fs::read_to_string(dir.path().join(".moon/cache/states/123/snapshot.json")).unwrap(),
            "\"content\""
        );

        dir.close().unwrap();
    }
}

mod cache_run_target_state {
    use super::*;

    #[test]
    #[serial]
    fn creates_parent_dir_on_call() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = cache.cache_run_target_state("foo:bar").unwrap();

        assert!(!item.path.exists());
        assert!(item.path.parent().unwrap().exists());

        dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn loads_cache_if_it_exists() {
        let dir = create_temp_dir();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = cache.cache_run_target_state("foo:bar").unwrap();

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

    #[test]
    #[serial]
    fn loads_cache_if_it_exists_and_cache_is_readonly() {
        let dir = create_temp_dir();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = run_with_env("read", || cache.cache_run_target_state("foo:bar")).unwrap();

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

    #[test]
    #[serial]
    fn loads_cache_if_it_exists_and_cache_is_readwrite() {
        let dir = create_temp_dir();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = run_with_env("read", || cache.cache_run_target_state("foo:bar")).unwrap();

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

    #[test]
    #[serial]
    fn doesnt_load_if_it_exists_but_cache_is_off() {
        let dir = create_temp_dir();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = run_with_env("off", || cache.cache_run_target_state("foo:bar")).unwrap();

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

    #[test]
    #[serial]
    fn doesnt_load_if_it_exists_but_cache_is_writeonly() {
        let dir = create_temp_dir();

        dir.child(".moon/cache/states/foo/bar/lastRun.json")
                .write_str(r#"{"exitCode":123,"hash":"","lastRunTime":0,"stderr":"","stdout":"","target":"foo:bar"}"#)
                .unwrap();

        let cache = CacheEngine::load(dir.path()).unwrap();
        let item = run_with_env("write", || cache.cache_run_target_state("foo:bar")).unwrap();

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

    #[test]
    #[serial]
    fn saves_to_cache() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").unwrap();

        item.exit_code = 123;

        run_with_env("", || item.save()).unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"exitCode":123,"hash":"","lastRunTime":0,"target":"foo:bar"}"#
        );

        dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn saves_to_cache_if_writeonly() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").unwrap();

        item.exit_code = 123;

        run_with_env("write", || item.save()).unwrap();

        assert_eq!(
            fs::read_to_string(item.path).unwrap(),
            r#"{"exitCode":123,"hash":"","lastRunTime":0,"target":"foo:bar"}"#
        );

        dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn doesnt_save_if_cache_off() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").unwrap();

        item.exit_code = 123;

        run_with_env("off", || item.save()).unwrap();

        assert!(!item.path.exists());

        dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn doesnt_save_if_cache_readonly() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let mut item = cache.cache_run_target_state("foo:bar").unwrap();

        item.exit_code = 123;

        run_with_env("read", || item.save()).unwrap();

        assert!(!item.path.exists());

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

    #[test]
    #[serial]
    fn creates_hash_file() {
        let dir = create_temp_dir();
        let cache = CacheEngine::load(dir.path()).unwrap();
        let hasher = TestHasher::default();

        cache.create_hash_manifest("abc123", &hasher).unwrap();

        assert!(cache.hashes_dir.join("abc123.json").exists());

        dir.close().unwrap();
    }
}
