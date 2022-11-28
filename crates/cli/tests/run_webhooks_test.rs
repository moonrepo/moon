use moon_config::NotifierConfig;
use moon_notifier::WebhookPayload;
use moon_test_utils::{create_sandbox_with_config, get_node_fixture_configs, Sandbox};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sandbox(uri: String) -> Sandbox {
    let (mut workspace_config, toolchain_config, projects_config) = get_node_fixture_configs();

    workspace_config.notifier = NotifierConfig {
        webhook_url: Some(format!("{}/webhook", uri)),
    };

    let sandbox = create_sandbox_with_config(
        "node",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&projects_config),
    );

    sandbox.enable_git();
    sandbox
}

#[tokio::test]
async fn sends_webhooks() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .expect(19)
        .mount(&server)
        .await;

    let mut sandbox = sandbox(server.uri());

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    assert.success();
}

#[tokio::test]
async fn sends_webhooks_for_cache_events() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .expect(37)
        .mount(&server)
        .await;

    let mut sandbox = sandbox(server.uri());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    // Run again to hit the cache
    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    assert.success();
}

#[tokio::test]
async fn doesnt_send_webhooks_if_first_fails() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let mut sandbox = sandbox(server.uri());

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    assert.failure();
}

#[tokio::test]
async fn all_webhooks_have_same_uuid() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut sandbox = sandbox(server.uri());

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    let received_requests = server.received_requests().await.unwrap();
    let mut uuid = None;

    for request in received_requests {
        let payload: WebhookPayload<String> =
            serde_json::from_str(&String::from_utf8(request.body).unwrap()).unwrap();

        if uuid.is_none() {
            uuid = Some(payload.uuid);
        } else {
            assert_eq!(&payload.uuid, uuid.as_ref().unwrap());
        }
    }
}
