#[async_trait::async_trait]
pub trait Describable<'tool>: Send + Sync {
    /// Return a unique identifier of the tool that'll be used in variables and file names.
    fn get_id(&self) -> &str;

    /// Return a loggable target name.
    fn get_log_target(&self) -> &str;

    /// Return a human readable name of the tool.
    fn get_name(&self) -> String;
}
