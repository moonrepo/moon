use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_api::Launchpad;
use moon_common::is_ci;
use moon_config::ToolchainsConfig;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct TelemetrySubscriber {
    toolchains_config: Arc<ToolchainsConfig>,
}

impl TelemetrySubscriber {
    pub fn new(toolchains_config: Arc<ToolchainsConfig>) -> Self {
        Self { toolchains_config }
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

            for (id, plugin) in &self.toolchains_config.plugins {
                if let Some(locator) = &plugin.plugin {
                    toolchains.insert(id.to_string(), locator.to_string());
                }
            }

            if !toolchains.is_empty()
                && let Some(launchpad) = Launchpad::instance()
            {
                let _ = launchpad.track_toolchain_usage(toolchains).await;
            }
        }

        Ok(())
    }
}
