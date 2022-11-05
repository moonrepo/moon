mod utils;

use moon_notifier::WebhookPayload;
use moon_utils::test::{create_moon_command, create_sandbox_with_git};
use utils::append_workspace_config;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn sends_webhooks() {
    let server = MockServer::start().await;
    let fixture = create_sandbox_with_git("node");

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .expect(19)
        .mount(&server)
        .await;

    append_workspace_config(
        fixture.path(),
        &format!("notifier:\n  webhookUrl: '{}/webhook'", server.uri()),
    );

    create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();
}

#[tokio::test]
async fn sends_webhooks_for_cache_events() {
    let server = MockServer::start().await;
    let fixture = create_sandbox_with_git("node");

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .expect(37)
        .mount(&server)
        .await;

    append_workspace_config(
        fixture.path(),
        &format!("notifier:\n  webhookUrl: '{}/webhook'", server.uri()),
    );

    create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();

    // Run again to hit the cache
    create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();
}

#[tokio::test]
async fn doesnt_send_webhooks_if_first_fails() {
    let server = MockServer::start().await;
    let fixture = create_sandbox_with_git("node");

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    append_workspace_config(
        fixture.path(),
        &format!("notifier:\n  webhookUrl: '{}/webhook'", server.uri()),
    );

    create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();
}

#[tokio::test]
async fn all_webhooks_have_same_uuid() {
    let server = MockServer::start().await;
    let fixture = create_sandbox_with_git("node");

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    append_workspace_config(
        fixture.path(),
        &format!("notifier:\n  webhookUrl: '{}/webhook'", server.uri()),
    );

    create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();

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
