use moon_cache::*;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_env_var::GlobalEnvBag;
use starbase_sandbox::{Sandbox, create_empty_sandbox};

fn create_engine(sandbox: &Sandbox) -> CacheEngine {
    CacheEngine::new(CacheContext::new(sandbox.path())).unwrap()
}

mod cache_engine {
    use super::*;

    #[test]
    fn creates_cache_dir_tag() {
        let sandbox = create_empty_sandbox();

        create_engine(&sandbox);

        assert!(sandbox.path().join(".moon/cache/CACHEDIR.TAG").exists());
    }

    #[test]
    fn removes_legacy_cache_dirs() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "{}");
        sandbox.create_file(".moon/cache/outputs/abc.tar.gz", "");
        sandbox.create_file(".moon/cache/states/project/task/lastRun.json", "{}");

        create_engine(&sandbox);

        assert!(!sandbox.path().join(".moon/cache/hashes").exists());
        assert!(!sandbox.path().join(".moon/cache/outputs").exists());
        assert!(
            sandbox
                .path()
                .join(".moon/cache/states/project/task/lastRun.json")
                .exists()
        );
    }

    #[test]
    fn returns_default_if_cache_missing() {
        let sandbox = create_empty_sandbox();
        let engine = create_engine(&sandbox);
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

        let engine = create_engine(&sandbox);
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
        let engine = create_engine(&sandbox);
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
        let engine = create_engine(&sandbox);
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

    mod hash_files {
        use super::*;

        fn rel(path: &str) -> WorkspaceRelativePathBuf {
            WorkspaceRelativePathBuf::from(path)
        }

        #[tokio::test]
        async fn hashes_files_into_hex() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "hello");
            sandbox.create_file("b.txt", "world");

            let engine = create_engine(&sandbox);

            let files = vec![rel("a.txt"), rel("b.txt")];
            let result = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(result.len(), 2);
            for path in &files {
                let hash = result.get(path).expect("missing hash");
                assert_eq!(hash.len(), 64);
                assert!(
                    hash.chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
                );
            }
        }

        #[tokio::test]
        async fn same_content_produces_same_hash() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "same");
            sandbox.create_file("b.txt", "same");

            let engine = create_engine(&sandbox);

            let result = engine
                .hash_files(sandbox.path(), &[rel("a.txt"), rel("b.txt")])
                .await
                .unwrap();

            assert_eq!(result[&rel("a.txt")], result[&rel("b.txt")]);
        }

        #[tokio::test]
        async fn different_content_produces_different_hash() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "one");
            sandbox.create_file("b.txt", "two");

            let engine = create_engine(&sandbox);

            let result = engine
                .hash_files(sandbox.path(), &[rel("a.txt"), rel("b.txt")])
                .await
                .unwrap();

            assert_ne!(result[&rel("a.txt")], result[&rel("b.txt")]);
        }

        #[tokio::test]
        async fn returns_empty_map_for_empty_input() {
            let sandbox = create_empty_sandbox();
            let engine = create_engine(&sandbox);

            let result = engine.hash_files(sandbox.path(), &[]).await.unwrap();

            assert!(result.is_empty());
        }

        #[tokio::test]
        async fn skips_missing_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("exists.txt", "here");

            let engine = create_engine(&sandbox);

            let result = engine
                .hash_files(
                    sandbox.path(),
                    &[
                        rel("exists.txt"),
                        rel("missing.txt"),
                        rel("nested/gone.txt"),
                    ],
                )
                .await
                .unwrap();

            assert_eq!(result.len(), 1);
            assert!(result.contains_key(&rel("exists.txt")));
        }

        fn backdate(path: &std::path::Path, secs: u64) {
            let modified = std::time::SystemTime::now() - std::time::Duration::from_secs(secs);
            std::fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(modified)
                .unwrap();
        }

        #[tokio::test]
        async fn memoizes_hashes_of_settled_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "hello");
            backdate(&sandbox.path().join("a.txt"), 60);

            let engine = create_engine(&sandbox);
            let files = [rel("a.txt")];

            let first = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(engine.file_hasher.memoized_count(), 1);

            let second = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(first, second);
        }

        #[tokio::test]
        async fn does_not_memoize_recently_modified_files() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "hello");

            let engine = create_engine(&sandbox);
            let files = [rel("a.txt")];

            engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(engine.file_hasher.memoized_count(), 0);

            // Same length, so only the content distinguishes them
            sandbox.create_file("a.txt", "world");

            let changed = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(
                changed[&rel("a.txt")],
                "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7"
            );
        }

        #[tokio::test]
        async fn rehashes_memoized_files_when_modified() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("a.txt", "hello");
            backdate(&sandbox.path().join("a.txt"), 60);

            let engine = create_engine(&sandbox);
            let files = [rel("a.txt")];

            let before = engine.hash_files(sandbox.path(), &files).await.unwrap();

            sandbox.create_file("a.txt", "world");

            let after = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_ne!(before[&rel("a.txt")], after[&rel("a.txt")]);
        }

        #[tokio::test]
        async fn hashes_large_files_across_buffers() {
            let sandbox = create_empty_sandbox();
            let content = "x".repeat(64 * 1024 * 3 + 123);
            sandbox.create_file("big.txt", &content);

            let engine = create_engine(&sandbox);

            let result = engine
                .hash_files(sandbox.path(), &[rel("big.txt")])
                .await
                .unwrap();

            assert_eq!(
                result[&rel("big.txt")],
                starbase_utils::hash::sha256::from_bytes(content.as_bytes())
            );
        }

        #[tokio::test]
        async fn skips_directories() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("file.txt", "content");
            std::fs::create_dir_all(sandbox.path().join("subdir")).unwrap();

            let engine = create_engine(&sandbox);

            let result = engine
                .hash_files(sandbox.path(), &[rel("file.txt"), rel("subdir")])
                .await
                .unwrap();

            assert_eq!(result.len(), 1);
            assert!(result.contains_key(&rel("file.txt")));
            assert!(!result.contains_key(&rel("subdir")));
        }

        #[tokio::test]
        async fn hashes_files_in_nested_directories() {
            let sandbox = create_empty_sandbox();
            sandbox.create_file("top.txt", "a");
            sandbox.create_file("nested/mid.txt", "b");
            sandbox.create_file("nested/deeper/bottom.txt", "c");

            let engine = create_engine(&sandbox);

            let files = vec![
                rel("top.txt"),
                rel("nested/mid.txt"),
                rel("nested/deeper/bottom.txt"),
            ];
            let result = engine.hash_files(sandbox.path(), &files).await.unwrap();

            assert_eq!(result.len(), 3);
            for path in &files {
                assert!(result.contains_key(path));
            }
        }
    }
}
