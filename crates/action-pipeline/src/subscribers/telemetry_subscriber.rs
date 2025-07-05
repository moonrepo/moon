use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_api::Launchpad;
use moon_common::is_ci;
use moon_config::ToolchainConfig;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct TelemetrySubscriber {
    toolchain_config: Arc<ToolchainConfig>,
}

impl TelemetrySubscriber {
    pub fn new(toolchain_config: Arc<ToolchainConfig>) -> Self {
        Self { toolchain_config }
    }
}

#[async_trait]
impl Subscriber for TelemetrySubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        // Only capture toolchain usage in CI
        if !is_ci() {
            return Ok(());
        }

        if matches!(event, Event::PipelineStarted { .. }) {
            let mut toolchains = BTreeMap::default();

            for (id, plugin) in &self.toolchain_config.plugins {
                if let Some(locator) = &plugin.plugin {
                    toolchains.insert(id.to_string(), locator.to_string());
                }
            }

            for platform in self.toolchain_config.get_enabled_platforms() {
                toolchains.insert(platform.to_string().to_lowercase(), "legacy".to_owned());
            }

            if !toolchains.is_empty() {
                let _ = Launchpad::instance()
                    .track_toolchain_usage(toolchains)
                    .await;
            }
        }

        Ok(())
    }
}
