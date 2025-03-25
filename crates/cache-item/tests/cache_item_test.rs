use moon_cache_item::*;
use moon_env_var::GlobalEnvBag;
use serial_test::serial;
use starbase_sandbox::{create_empty_sandbox, create_sandbox};
use std::path::Path;

fn run_with_mode<T, F>(mode: CacheMode, callback: F) -> T
where
    F: FnOnce() -> T,
{
    let bag = GlobalEnvBag::instance();

    bag.set(
        "MOON_CACHE",
        match mode {
            CacheMode::Off => "off",
            CacheMode::Read => "read",
            CacheMode::ReadWrite => "read-write",
            CacheMode::Write => "write",
        },
    );

    let result = callback();

    bag.remove("MOON_CACHE");

    result
}

fn load_item(sandbox: &Path) -> CacheItem<CommonCacheState> {
    CacheItem::<CommonCacheState>::load(sandbox.join("data.json")).unwrap()
}

fn load_item_with_mode(mode: CacheMode, sandbox: &Path) -> CacheItem<CommonCacheState> {
    run_with_mode(mode, || load_item(sandbox))
}

mod cache_item {
    use super::*;

    #[serial]
    fn can_omit_file_extension() {
        let sandbox = create_sandbox("item");
        let item = CacheItem::<CommonCacheState>::load(sandbox.path().join("data")).unwrap();

        assert_eq!(item.data.last_hash, "abc123");
        assert!(!sandbox.path().join("data").exists());
    }

    #[serial]
    fn will_change_file_extension() {
        let sandbox = create_sandbox("item");
        let item = CacheItem::<CommonCacheState>::load(sandbox.path().join("data.yml")).unwrap();

        assert_eq!(item.data.last_hash, "abc123");
        assert!(!sandbox.path().join("data.yml").exists());
    }

    #[test]
    fn loads_defaults_when_missing() {
        let sandbox = create_empty_sandbox();
        let item = load_item(sandbox.path());

        assert_eq!(item.data.last_hash, "");
    }

    mod off {
        use super::*;

        #[test]
        #[serial]
        fn loads_defaults() {
            let sandbox = create_sandbox("item");
            let item = load_item_with_mode(CacheMode::Off, sandbox.path());

            assert_eq!(item.data.last_hash, "");
        }

        #[test]
        #[serial]
        fn doesnt_save() {
            let sandbox = create_sandbox("item");

            run_with_mode(CacheMode::Off, || {
                let mut item = load_item(sandbox.path());
                item.data.last_hash = "xyz789".into();
                item.save().unwrap();
            });

            assert_eq!(load_item(sandbox.path()).data.last_hash, "abc123");
        }
    }

    mod read {
        use super::*;

        #[test]
        #[serial]
        fn loads_file_contents() {
            let sandbox = create_sandbox("item");
            let item = load_item_with_mode(CacheMode::Read, sandbox.path());

            assert_eq!(item.data.last_hash, "abc123");
        }

        #[test]
        #[serial]
        fn doesnt_save() {
            let sandbox = create_sandbox("item");

            run_with_mode(CacheMode::Read, || {
                let mut item = load_item(sandbox.path());
                item.data.last_hash = "xyz789".into();
                item.save().unwrap();
            });

            assert_eq!(load_item(sandbox.path()).data.last_hash, "abc123");
        }
    }

    mod read_write {
        use super::*;

        #[test]
        #[serial]
        fn loads_file_contents() {
            let sandbox = create_sandbox("item");
            let item = load_item_with_mode(CacheMode::ReadWrite, sandbox.path());

            assert_eq!(item.data.last_hash, "abc123");
        }

        #[test]
        #[serial]
        fn saves_file_contents() {
            let sandbox = create_sandbox("item");

            run_with_mode(CacheMode::ReadWrite, || {
                let mut item = load_item(sandbox.path());
                item.data.last_hash = "xyz789".into();
                item.save().unwrap();
            });

            assert_eq!(load_item(sandbox.path()).data.last_hash, "xyz789");
        }
    }

    mod write {
        use super::*;

        #[test]
        #[serial]
        fn loads_defaults_and_doesnt_read() {
            let sandbox = create_sandbox("item");
            let item = load_item_with_mode(CacheMode::Write, sandbox.path());

            assert_eq!(item.data.last_hash, "");
        }

        #[test]
        #[serial]
        fn saves_file_contents() {
            let sandbox = create_sandbox("item");

            run_with_mode(CacheMode::Write, || {
                let mut item = load_item(sandbox.path());
                item.data.last_hash = "xyz789".into();
                item.save().unwrap();
            });

            assert_eq!(load_item(sandbox.path()).data.last_hash, "xyz789");
        }
    }
}
