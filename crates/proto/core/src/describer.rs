#[async_trait::async_trait]
pub trait Describable<'tool>: Send + Sync {
    /// Return a loggable target name.
    fn get_log_target(&self) -> &str;

    /// Return a human readable name of the tool.
    fn get_name(&self) -> String;
}
