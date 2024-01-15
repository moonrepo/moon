use moon_pdk_api::extension::*;
use moon_plugin::{create_plugin_manifest, Id, Plugin, PluginContainer, PluginType};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct ExtensionPlugin {
    pub id: Id,
    pub type_of: PluginType,

    plugin: PluginContainer,
}

impl ExtensionPlugin {
    pub fn execute(&self) -> miette::Result<()> {
        self.plugin.call_func_without_output(
            "execute_extension",
            ExtensionContext {
                // TODO
                working_dir: self.plugin.to_virtual_path(PathBuf::new()),
                workspace_root: self.plugin.to_virtual_path(PathBuf::new()),
            },
        )?;

        Ok(())
    }
}

impl Plugin for ExtensionPlugin {
    fn new(id: Id, wasm_file: PathBuf) -> miette::Result<Self> {
        Ok(Self {
            type_of: PluginType::Extension,
            plugin: PluginContainer::new_without_functions(
                id.clone(),
                create_plugin_manifest(wasm_file, BTreeMap::new()),
            )?,
            id,
        })
    }

    fn get_type(&self) -> PluginType {
        self.type_of
    }
}
