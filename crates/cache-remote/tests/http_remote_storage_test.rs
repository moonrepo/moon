use httpmock::prelude::*;
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache_remote::HttpRemoteStorage;
use moon_cache_storage::{CacheContext, Manifest, StorageBackend};
use moon_config::{CacheConfig, RemoteConfig};
use moon_hash::Digest;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::sync::Arc;

const INSTANCE: &str = "moon-test";

fn create_storage(sandbox: &Sandbox, host: String) -> HttpRemoteStorage {
    let mut remote = RemoteConfig {
        host: Some(host),
        ..Default::default()
    };
    remote.cache.instance_name = INSTANCE.to_owned();

    let context = CacheContext {
        cache_dir: sandbox.path().join(".moon/cache"),
        cache_config: Arc::new(CacheConfig::default()),
        config_dir: sandbox.path().join(".moon"),
        remote_config: Arc::new(remote),
        remote_debug: false,
        workspace_root: sandbox.path().to_path_buf(),
    };

    HttpRemoteStorage::new(context).unwrap()
}

fn digest_of(bytes: &[u8]) -> Digest {
    Digest::from_bytes(bytes).unwrap()
}

mod http_remote_storage {
    use super::*;

    mod connect {
        use super::*;

        #[tokio::test]
        async fn enables_backend_when_status_ok() {
            let server = MockServer::start_async().await;
            let status = server.mock(|when, then| {
                when.method(GET).path("/status");
                then.status(200);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            storage.connect().await.unwrap();

            status.assert_calls_async(1).await;
            assert!(storage.is_enabled());
        }

        #[tokio::test]
        async fn tolerates_404_status() {
            // The status endpoint is non-standard, so a 404 must not disable
            // the backend.
            let server = MockServer::start_async().await;
            server.mock(|when, then| {
                when.method(GET).path("/status");
                then.status(404);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            storage.connect().await.unwrap();

            assert!(storage.is_enabled());
        }

        #[tokio::test]
        async fn errors_on_unexpected_status() {
            let server = MockServer::start_async().await;
            server.mock(|when, then| {
                when.method(GET).path("/status");
                then.status(500);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            assert!(storage.connect().await.is_err());
            assert!(!storage.is_enabled());
        }
    }

    mod manifests {
        use super::*;

        #[tokio::test]
        async fn stores_manifest() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            let mock = server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(200);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            storage
                .store_manifest(digest, Manifest::default())
                .await
                .unwrap();

            mock.assert_calls_async(1).await;
        }

        #[tokio::test]
        async fn retrieves_manifest() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            let manifest = Manifest {
                exit_code: 7,
                ..Default::default()
            };
            let body = serde_json::to_string(&manifest).unwrap();
            let mock = server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(200)
                    .header("content-type", "application/json")
                    .body(body);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let result = storage.retrieve_manifest(digest).await.unwrap();

            mock.assert_calls_async(1).await;
            assert_eq!(result.unwrap().exit_code, 7);
        }

        #[tokio::test]
        async fn retrieve_returns_none_on_404() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"missing");
            server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(404);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            assert!(storage.retrieve_manifest(digest).await.unwrap().is_none());
        }

        #[tokio::test]
        async fn retrieve_errors_on_server_error() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"boom");
            server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(500);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            assert!(storage.retrieve_manifest(digest).await.is_err());
        }
    }

    mod blobs {
        use super::*;

        #[tokio::test]
        async fn stores_inline_blob() {
            let server = MockServer::start_async().await;
            let content = b"inline blob";
            let digest = digest_of(content);
            let mock = server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(200);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let source = BlobInput {
                content: BlobContent::Inline(Bytes::from_static(content)),
                digest: digest.clone(),
            };
            let stored = storage.store_blobs(vec![source], false).await.unwrap();

            mock.assert_calls_async(1).await;
            assert_eq!(stored, vec![digest]);
        }

        #[tokio::test]
        async fn stores_file_blob() {
            let server = MockServer::start_async().await;
            let content = b"file blob content";
            let digest = digest_of(content);
            let mock = server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(200);
            });
            let sandbox = create_empty_sandbox();
            sandbox.create_file("blob.txt", "file blob content");
            let storage = create_storage(&sandbox, server.base_url());

            let source = BlobInput {
                content: BlobContent::File("blob.txt".into()),
                digest: digest.clone(),
            };
            let stored = storage.store_blobs(vec![source], false).await.unwrap();

            mock.assert_calls_async(1).await;
            assert_eq!(stored, vec![digest]);
        }

        #[tokio::test]
        async fn store_errors_on_server_error() {
            let server = MockServer::start_async().await;
            let content = b"nope";
            let digest = digest_of(content);
            server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(500);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let source = BlobInput {
                content: BlobContent::Inline(Bytes::from_static(content)),
                digest,
            };

            assert!(storage.store_blobs(vec![source], false).await.is_err());
        }

        #[tokio::test]
        async fn retrieves_blobs() {
            let server = MockServer::start_async().await;
            let content = "downloaded";
            let digest = digest_of(content.as_bytes());
            let mock = server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(200).body(content);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let blobs = storage.retrieve_blobs(vec![digest], false).await.unwrap();

            mock.assert_calls_async(1).await;
            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].bytes.as_ref(), content.as_bytes());
        }

        #[tokio::test]
        async fn find_missing_assumes_all_missing() {
            // The HTTP API has no batch-existence query, so every digest is
            // reported as missing.
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, "http://127.0.0.1:0".to_owned());

            let a = digest_of(b"a");
            let b = digest_of(b"b");
            let missing = storage
                .find_missing_blobs(vec![a.clone(), b.clone()])
                .await
                .unwrap();

            assert_eq!(missing.len(), 2);
            assert!(missing.contains(&a));
            assert!(missing.contains(&b));
        }
    }
}
