use httpmock::prelude::*;
use moon_config::NotifierConfig;
use moon_test_utils::{create_sandbox_with_config, get_node_fixture_configs, Sandbox};

fn sandbox(uri: String) -> Sandbox {
    let (mut workspace_config, toolchain_config, tasks_config) = get_node_fixture_configs();

    workspace_config.notifier = NotifierConfig {
        webhook_url: Some(format!("{uri}/webhook")),
    };

    let sandbox = create_sandbox_with_config(
        "node",
        Some(&workspace_config),
        Some(&toolchain_config),
        Some(&tasks_config),
    );

    sandbox.enable_git();
    sandbox
}

#[tokio::test]
async fn sends_webhooks() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/webhook");
        then.status(200);
    });

    let sandbox = sandbox(server.url(""));

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    mock.assert_hits(19);

    assert.success();
}

#[tokio::test]
async fn sends_webhooks_for_cache_events() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/webhook");
        then.status(200);
    });

    let sandbox = sandbox(server.url(""));

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    // Run again to hit the cache
    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    mock.assert_hits(37);

    assert.success();
}

#[tokio::test]
async fn doesnt_send_webhooks_if_first_fails() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST).path("/webhook");
        then.status(500);
    });

    let sandbox = sandbox(server.url(""));

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    mock.assert_hits(1);
}

#[tokio::test]
async fn all_webhooks_have_same_uuid() {
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/webhook")
            .json_body_partial(r#"{"uuid":"XXXX-XXXX-XXXX-XXXX"}"#);

        then.status(200);
    });

    let sandbox = sandbox(server.url(""));

    sandbox.run_moon(|cmd| {
        cmd.arg("run").arg("node:cjs");
    });

    mock.assert_hits(19);
}
