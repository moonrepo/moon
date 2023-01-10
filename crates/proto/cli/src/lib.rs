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

pub fn create_tool(tool: ToolType, version: &str) -> Result<Box<dyn Tool>, ProtoError> {
    let proto = Proto::new()?;

    Ok(match tool {
        // Node.js
        ToolType::Node => Box::new(node::NodeLanguage::new(&proto, Some(version))),
        ToolType::Npm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Npm,
            Some(version),
        )),
        ToolType::Pnpm => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Pnpm,
            Some(version),
        )),
        ToolType::Yarn => Box::new(node::NodeDependencyManager::new(
            &proto,
            node::NodeDependencyManagerType::Yarn,
            Some(version),
        )),
    })
}
