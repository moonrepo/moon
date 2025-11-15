mod utils;

use httpmock::prelude::*;
use moon_config::PartialNotifierConfig;
use moon_test_utils2::MoonSandbox;
use utils::create_tasks_sandbox;

fn create_webhooks_sandbox(url: String) -> MoonSandbox {
    let sandbox = create_tasks_sandbox();
    sandbox.enable_git();
    sandbox.update_workspace_config(|config| {
        config.notifier = Some(PartialNotifierConfig {
            terminal_notifications: None,
            webhook_url: Some(url),
            webhook_acknowledge: Some(false),
        });
    });
    sandbox
}

mod run_webhooks {
    use super::*;

    #[tokio::test]
    async fn sends_webhooks() {
        let server = MockServer::start_async().await;

        let mock = server.mock(|when, then| {
            when.method(POST).path("/webhook");
            then.status(200).body("{}");
        });

        let sandbox = create_webhooks_sandbox(server.url("/webhook"));

        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node:base");
            })
            .success();

        mock.assert_calls_async(20).await;
    }

    #[tokio::test]
    async fn sends_webhooks_for_cache_events() {
        let server = MockServer::start_async().await;

        let mock = server.mock(|when, then| {
            when.method(POST).path("/webhook");
            then.status(200);
        });

        let sandbox = create_webhooks_sandbox(server.url("/webhook"));

        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node:base");
            })
            .debug();

        // Run again to hit the cache
        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node:base");
            })
            .debug();

        mock.assert_calls_async(40).await;
    }

    #[tokio::test]
    async fn doesnt_send_webhooks_if_first_fails() {
        let server = MockServer::start_async().await;

        let mock = server.mock(|when, then| {
            when.method(POST).path("/webhook");
            then.status(500);
        });

        let sandbox = create_webhooks_sandbox(server.url("/webhook"));

        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node:base");
            })
            .success();

        mock.assert_calls_async(1).await;
    }

    #[tokio::test]
    async fn all_webhooks_have_same_uuid() {
        let server = MockServer::start_async().await;

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/webhook")
                .json_body_includes(r#"{"uuid":"XXXX-XXXX-XXXX-XXXX"}"#);

            then.status(200);
        });

        let sandbox = create_webhooks_sandbox(server.url("/webhook"));

        sandbox
            .run_bin(|cmd| {
                cmd.arg("run").arg("node:base");
            })
            .success();

        mock.assert_calls_async(20).await;
    }
}
