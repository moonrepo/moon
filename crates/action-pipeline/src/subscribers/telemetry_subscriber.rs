use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_api::Launchpad;
use moon_common::is_ci;
use moon_config::ToolchainConfig;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct TelemetrySubscriber {
    toolchain_config: Arc<ToolchainConfig>,
    requests: Vec<JoinHandle<miette::Result<()>>>,
}

impl TelemetrySubscriber {
    pub fn new(toolchain_config: Arc<ToolchainConfig>) -> Self {
        Self {
            toolchain_config,
            requests: vec![],
        }
    }
}

#[async_trait]
impl Subscriber for TelemetrySubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        // Only capture toolchain usage in CI
        if !is_ci() {
            return Ok(());
        }

        match event {
            Event::PipelineStarted { .. } => {
                for (id, plugin) in &self.toolchain_config.plugins {
                    if let Some(locator) = &plugin.plugin {
                        let id = id.to_string();
                        let locator = locator.to_string();

                        self.requests.push(tokio::spawn(async move {
                            Launchpad::instance()
                                .track_toolchain_usage(id, locator)
                                .await
                        }));
                    }
                }

                for platform in self.toolchain_config.get_enabled_platforms() {
                    let id = platform.to_string().to_lowercase();
                    let locator = "legacy".to_owned();

                    self.requests.push(tokio::spawn(async move {
                        Launchpad::instance()
                            .track_toolchain_usage(id, locator)
                            .await
                    }));
                }
            }
            Event::PipelineCompleted { .. } => {
                for future in self.requests.drain(0..) {
                    // Ignore telemetry errors
                    let _ = future.await;
                }
            }
            _ => {}
        };

        Ok(())
    }
}
