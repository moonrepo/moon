use crate::event_emitter::{Event, Subscriber};
use async_trait::async_trait;
use moon_action::ActionPipelineStatus;
use moon_config::NotifierEventType;
use moon_config::patterns::Regex;
use moon_notifier::notify_terminal;
use moon_time::elapsed;

pub struct NotificationsSubscriber {
    ansi: Regex,
    toast: NotifierEventType,
}

impl NotificationsSubscriber {
    pub fn new(toast: NotifierEventType) -> Self {
        NotificationsSubscriber {
            ansi: Regex::new(r"\x1b\[([\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e])").unwrap(),
            toast,
        }
    }
}

#[async_trait]
impl Subscriber for NotificationsSubscriber {
    async fn on_emit<'data>(&mut self, event: &Event<'data>) -> miette::Result<()> {
        if self.toast == NotifierEventType::Never {
            return Ok(());
        }

        match event {
            Event::PipelineCompleted {
                actions,
                duration,
                error,
                status,
                ..
            } => {
                if error.is_none()
                    && matches!(
                        self.toast,
                        NotifierEventType::Always | NotifierEventType::Success
                    )
                {
                    notify_terminal(
                        "Pipeline successful",
                        format!(
                            "Pipeline has successfully ran {} actions in {}.",
                            actions.len(),
                            elapsed(duration.unwrap_or_default())
                        ),
                    )?;
                }

                if matches!(
                    self.toast,
                    NotifierEventType::Always | NotifierEventType::Failure
                ) && let Some(error) = error
                {
                    notify_terminal(
                        match status {
                            ActionPipelineStatus::Aborted => "Pipeline aborted",
                            ActionPipelineStatus::Interrupted => "Pipeline interrupted",
                            ActionPipelineStatus::Terminated => "Pipeline terminated",
                            _ => "Pipeline failed",
                        },
                        self.ansi.replace_all(error, ""),
                    )?;
                }
            }
            Event::TaskRan { error, target, .. } => {
                if matches!(self.toast, NotifierEventType::TaskFailure)
                    && let Some(error) = error
                {
                    notify_terminal(
                        format!("Task {target} failed"),
                        self.ansi.replace_all(error, ""),
                    )?;
                }
            }
            _ => {}
        };

        Ok(())
    }
}
