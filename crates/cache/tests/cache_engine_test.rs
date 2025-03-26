use moon_cache::*;
use moon_env_var::GlobalEnvBag;
use starbase_sandbox::create_empty_sandbox;

mod cache_engine {
    use super::*;

    #[test]
    fn creates_cache_dir_tag() {
        let sandbox = create_empty_sandbox();

        CacheEngine::new(sandbox.path()).unwrap();

        assert!(sandbox.path().join(".moon/cache/CACHEDIR.TAG").exists());
    }

    #[test]
    fn returns_default_if_cache_missing() {
        let sandbox = create_empty_sandbox();
        let engine = CacheEngine::new(sandbox.path()).unwrap();
        let item = engine
            .state
            .load_state::<CommonCacheState>("state.json")
            .unwrap();

        assert_eq!(item.data, CommonCacheState::default());
    }

    #[test]
    fn reads_cache_if_exists() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            ".moon/cache/states/state.json",
            r#"{ "lastHash": "abc123" }"#,
        );

        let engine = CacheEngine::new(sandbox.path()).unwrap();
        let item = engine
            .state
            .load_state::<CommonCacheState>("state.json")
            .unwrap();

        assert_eq!(
            item.data,
            CommonCacheState {
                last_hash: "abc123".into()
            }
        );
    }

    #[test]
    fn can_write_cache_if_mode_off() {
        let sandbox = create_empty_sandbox();
        let engine = CacheEngine::new(sandbox.path()).unwrap();
        let bag = GlobalEnvBag::instance();

        bag.set("MOON_CACHE", "off");

        engine
            .write(
                "test.json",
                &CommonCacheState {
                    last_hash: "abc123".into(),
                },
            )
            .unwrap();

        assert!(sandbox.path().join(".moon/cache/test.json").exists());

        bag.remove("MOON_CACHE");
    }

    #[test]
    fn can_write_cache_if_mode_readonly() {
        let sandbox = create_empty_sandbox();
        let engine = CacheEngine::new(sandbox.path()).unwrap();
        let bag = GlobalEnvBag::instance();

        bag.set("MOON_CACHE", "read");

        engine
            .write(
                engine.state.resolve_path("test.json"),
                &CommonCacheState {
                    last_hash: "abc123".into(),
                },
            )
            .unwrap();

        assert!(sandbox.path().join(".moon/cache/states/test.json").exists());

        bag.remove("MOON_CACHE");
    }
}
