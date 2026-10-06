use httpmock::prelude::*;
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache_remote::HttpRemoteStorage;
use moon_cache_storage::{
    CacheContext, StorageBackend, TaskManifest, TaskManifestFile, TaskManifestSymlink,
};
use moon_config::RemoteConfig;
use moon_hash::Digest;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const INSTANCE: &str = "moon-test";

// Retries are disabled by default, so that failures don't wait on backoff
fn create_storage(sandbox: &Sandbox, host: String) -> HttpRemoteStorage {
    create_storage_with_retries(sandbox, host, 0)
}

fn create_storage_with_retries(
    sandbox: &Sandbox,
    host: String,
    retry_count: u8,
) -> HttpRemoteStorage {
    let mut remote = RemoteConfig {
        host: Some(host),
        ..Default::default()
    };
    remote.cache.instance_name = INSTANCE.to_owned();
    remote.cache.retry_count = retry_count;

    let mut context = CacheContext::new(sandbox.path());
    context.remote_config = Arc::new(remote);

    HttpRemoteStorage::new(context).unwrap()
}

// Responds to each request with the next status and body in the sequence,
// which httpmock can't do, as its mocks always respond the same way
async fn serve_sequence(responses: Vec<(u16, &'static str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![];
            let mut buffer = [0; 1024];

            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let size = stream.read(&mut buffer).await.unwrap();

                if size == 0 {
                    break;
                }

                request.extend_from_slice(&buffer[..size]);
            }

            // Close the connection so that each request opens a new one
            let response = format!(
                "HTTP/1.1 {status} STATUS\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );

            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        }
    });

    format!("http://{address}")
}

async fn get_unreachable_host() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    drop(listener);

    format!("http://{address}")
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
            assert!(storage.is_readable());
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

            assert!(storage.is_readable());
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
            assert!(!storage.is_readable());
        }

        #[tokio::test]
        async fn errors_when_host_unreachable() {
            // Otherwise every request would exhaust its retries before failing.
            // Covers both the retried and non-retried middleware errors.
            for retry_count in [0, 1] {
                let sandbox = create_empty_sandbox();
                let storage = create_storage_with_retries(
                    &sandbox,
                    get_unreachable_host().await,
                    retry_count,
                );

                assert!(storage.connect().await.is_err());
                assert!(!storage.is_readable());
            }
        }

        #[tokio::test]
        async fn errors_when_host_drops_connections() {
            // TEST-NET-1 is not routable, so the connection never completes
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, "http://192.0.2.1:8080".into(), 0);
            let start = std::time::Instant::now();

            assert!(storage.connect().await.is_err());
            assert!(!storage.is_readable());
            assert!(start.elapsed() < std::time::Duration::from_secs(10));
        }

        #[tokio::test]
        async fn retries_status_after_transient_error() {
            let host = serve_sequence(vec![(503, ""), (200, "")]).await;
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, host, 1);

            storage.connect().await.unwrap();

            assert!(storage.is_readable());
        }
    }

    mod manifests {
        use super::*;

        #[tokio::test]
        async fn stores_manifest_as_bazel_action_result() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            let file_digest = digest_of(b"file contents");
            let stdout_digest = digest_of(b"out");

            // The AC entry must go over the wire as an RE API `ActionResult` in
            // proto3 JSON form (camelCase keys, int64 sizes as strings), not as
            // moon's internal manifest format. The exact body match also proves
            // `stdoutRaw` stays empty on upload even though the manifest holds
            // the bytes — the raw fields are reserved for server responses, the
            // CAS digest carries the output.
            let expected_body = serde_json::json!({
                "exitCode": 2,
                "outputFiles": [{
                    "path": "out/file.txt",
                    "digest": {
                        "hash": file_digest.hash.to_string(),
                        "sizeBytes": file_digest.size.to_string(),
                    },
                    "isExecutable": true,
                    "nodeProperties": {},
                }],
                "outputSymlinks": [{
                    "path": "out/link",
                    "target": "out/file.txt",
                    "nodeProperties": {},
                }],
                "stdoutDigest": {
                    "hash": stdout_digest.hash.to_string(),
                    "sizeBytes": stdout_digest.size.to_string(),
                },
                "executionMetadata": {
                    "worker": "moon",
                },
            });

            let mock = server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash))
                    .header("content-type", "application/json")
                    .json_body(expected_body);
                then.status(200);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let manifest = TaskManifest {
                exit_code: 2,
                files: vec![TaskManifestFile {
                    digest: Some(file_digest),
                    is_executable: true,
                    path: "out/file.txt".into(),
                    ..Default::default()
                }],
                symlinks: vec![TaskManifestSymlink {
                    path: "out/link".into(),
                    target: "out/file.txt".into(),
                    ..Default::default()
                }],
                stdout_bytes: Some(Bytes::from_static(b"out")),
                stdout_digest: Some(stdout_digest),
                ..Default::default()
            };

            storage.store_task_manifest(digest, manifest).await.unwrap();

            mock.assert_calls_async(1).await;
        }

        #[tokio::test]
        async fn store_errors_on_server_error() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            server.mock(|when, then| {
                when.method(PUT)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(500);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            assert!(
                storage
                    .store_task_manifest(digest, TaskManifest::default())
                    .await
                    .is_err()
            );
        }

        #[tokio::test]
        async fn retrieves_manifest_from_bazel_action_result() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            let file_digest = digest_of(b"file contents");
            let stdout_digest = digest_of(b"out");
            let stderr_digest = digest_of(b"error");

            // A response in the shape a real RE server produces: proto3 JSON
            // with camelCase keys, int64 sizes as strings, raw output base64
            // encoded, and stderr provided only by digest (inlining the raw
            // output is optional for a server).
            let body = serde_json::json!({
                "exitCode": 7,
                "outputFiles": [{
                    "path": "out/file.txt",
                    "digest": {
                        "hash": file_digest.hash.to_string(),
                        "sizeBytes": file_digest.size.to_string(),
                    },
                    "isExecutable": true,
                    "nodeProperties": { "unixMode": 493 },
                }],
                "outputSymlinks": [{ "path": "out/link", "target": "out/file.txt" }],
                "stdoutRaw": "b3V0",
                "stdoutDigest": {
                    "hash": stdout_digest.hash.to_string(),
                    "sizeBytes": stdout_digest.size.to_string(),
                },
                "stderrDigest": {
                    "hash": stderr_digest.hash.to_string(),
                    "sizeBytes": stderr_digest.size.to_string(),
                },
            });
            let mock = server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash))
                    .header("accept", "application/json");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(body.to_string());
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            let manifest = storage
                .retrieve_task_manifest(digest)
                .await
                .unwrap()
                .unwrap();

            mock.assert_calls_async(1).await;
            assert_eq!(manifest.exit_code, 7);
            assert_eq!(manifest.files.len(), 1);
            assert_eq!(manifest.files[0].path.as_str(), "out/file.txt");
            assert_eq!(manifest.files[0].digest, Some(file_digest));
            assert!(manifest.files[0].is_executable);
            assert_eq!(manifest.files[0].unix_mode, Some(493));
            assert_eq!(manifest.symlinks.len(), 1);
            assert_eq!(manifest.symlinks[0].path.as_str(), "out/link");
            assert_eq!(manifest.symlinks[0].target.as_str(), "out/file.txt");
            // Inlined raw output is decoded; the non-inlined stream stays a
            // digest to be fetched from the CAS during hydration.
            assert_eq!(manifest.stdout_bytes, Some(Bytes::from_static(b"out")));
            assert_eq!(manifest.stdout_digest, Some(stdout_digest));
            assert!(manifest.stderr_bytes.is_none());
            assert_eq!(manifest.stderr_digest, Some(stderr_digest));
        }

        #[tokio::test]
        async fn retrieve_errors_on_legacy_manifest_body() {
            // Entries written by older moon versions stored moon's internal
            // manifest JSON in the AC. Those no longer parse as an
            // `ActionResult`, and must surface as an error instead of a bogus
            // cache hit.
            let server = MockServer::start_async().await;
            let digest = digest_of(b"action");
            let body = serde_json::to_string(&TaskManifest {
                exit_code: 7,
                ..Default::default()
            })
            .unwrap();
            server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/ac/{}", digest.hash));
                then.status(200)
                    .header("content-type", "application/json")
                    .body(body);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage(&sandbox, server.base_url());

            assert!(storage.retrieve_task_manifest(digest).await.is_err());
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

            assert!(
                storage
                    .retrieve_task_manifest(digest)
                    .await
                    .unwrap()
                    .is_none()
            );
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

            assert!(storage.retrieve_task_manifest(digest).await.is_err());
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
                content: BlobContent::File(sandbox.path().join("blob.txt")),
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
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content.as_bytes());
        }

        #[tokio::test]
        async fn retries_blob_download_after_transient_error() {
            let content = "downloaded";
            let digest = digest_of(content.as_bytes());
            let host = serve_sequence(vec![(404, ""), (503, ""), (502, ""), (200, content)]).await;
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, host, 2);

            storage.connect().await.unwrap();

            let blobs = storage.retrieve_blobs(vec![digest], false).await.unwrap();

            assert_eq!(blobs.len(), 1);
            assert_eq!(blobs[0].content.get_bytes().unwrap(), content.as_bytes());
        }

        #[tokio::test]
        async fn retries_blob_upload_after_transient_error() {
            let content = b"uploaded";
            let digest = digest_of(content);
            let host = serve_sequence(vec![(404, ""), (429, ""), (200, "")]).await;
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, host, 1);

            storage.connect().await.unwrap();

            let source = BlobInput {
                content: BlobContent::Inline(Bytes::from_static(content)),
                digest: digest.clone(),
            };
            let stored = storage.store_blobs(vec![source], false).await.unwrap();

            assert_eq!(stored, vec![digest]);
        }

        #[tokio::test]
        async fn errors_once_retries_are_exhausted() {
            let server = MockServer::start_async().await;
            let content = "unavailable";
            let digest = digest_of(content.as_bytes());
            let mock = server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(503);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, server.base_url(), 2);

            storage.connect().await.unwrap();

            assert!(storage.retrieve_blobs(vec![digest], false).await.is_err());

            mock.assert_calls_async(3).await;
        }

        #[tokio::test]
        async fn does_not_retry_client_errors() {
            let server = MockServer::start_async().await;
            let digest = digest_of(b"forbidden");
            let mock = server.mock(|when, then| {
                when.method(GET)
                    .path(format!("/{INSTANCE}/cas/{}", digest.hash));
                then.status(403);
            });
            let sandbox = create_empty_sandbox();
            let storage = create_storage_with_retries(&sandbox, server.base_url(), 2);

            storage.connect().await.unwrap();

            assert!(storage.retrieve_blobs(vec![digest], false).await.is_err());

            mock.assert_calls_async(1).await;
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
