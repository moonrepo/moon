use proto_core::warpgate::find_debug_locator_with_url_fallback;
use proto_core::{FileLocator, PluginLocator};

pub fn find_plugin_locator(name: &str, version: &str) -> PluginLocator {
    // TODO remove once v2 plugins are published
    #[cfg(debug_assertions)]
    {
        use std::env;
        use std::path::PathBuf;

        let prebuilts_dir = env::var("WASM_PREBUILTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let root = env::current_dir().unwrap();

                // repo root
                if root.join("wasm/prebuilts").exists() {
                    root.join("wasm/prebuilts")
                }
                // within a crate
                else {
                    root.join("../../wasm/prebuilts")
                }
            });
        let wasm_path = prebuilts_dir.join(format!("{name}.wasm"));

        if wasm_path.exists() {
            return PluginLocator::File(Box::new(FileLocator {
                file: format!("file://{}", wasm_path.display()),
                path: Some(wasm_path),
            }));
        }
    }

    find_debug_locator_with_url_fallback(name, version)
}
