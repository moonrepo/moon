use moon_tool::Tool;

#[derive(Debug, Default)]
pub struct SystemToolStub;

impl Tool for SystemToolStub {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
}
