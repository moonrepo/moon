use moon_cache::CacheEngine;
use moon_test_utils::create_temp_dir;
use serde::Serialize;
use serial_test::serial;
use std::env;

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
