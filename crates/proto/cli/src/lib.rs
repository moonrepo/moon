use clap::ValueEnum;
use proto_core::Tool;

pub use proto_core::*;
pub use proto_error::*;
pub use proto_node as node;

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum ToolType {
    // Node.js
    Node,
    Npm,
    Pnpm,
    Yarn,
}

pub fn create_tool(tool: ToolType) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let proto = Proto::new()?;

    Ok(match tool {
        // Node.js
        ToolType::Node => Box::new(node::NodeLanguage::new(&proto)),
        ToolType::Npm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Npm,
        )),
        ToolType::Pnpm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Pnpm,
        )),
        ToolType::Yarn => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Yarn,
        )),
    })
}
