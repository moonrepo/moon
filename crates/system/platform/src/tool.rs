use moon_tool::Tool;

pub struct SystemToolStub;

impl Tool for SystemToolStub {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
}
