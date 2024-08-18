#![allow(clippy::disallowed_types)]

mod wrappers;

use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use warpgate::PluginLoader;
use warpgate::{
    host::{create_host_functions, HostData},
    inject_default_manifest_config, test_utils, Id, PluginContainer, PluginManifest, Wasm,
};

pub use moon_pdk_api::*;
pub use wrappers::*;

pub fn create_plugin_container_with_config(
    id: &str,
    sandbox: &Path,
    config: HashMap<String, String>,
) -> PluginContainer {
    let id = Id::new(id).unwrap();
    let loader = PluginLoader::new(sandbox.join("plugins"), sandbox.join("temp"));
    let virtual_paths = BTreeMap::<PathBuf, PathBuf>::from_iter([
        (sandbox.to_path_buf(), "/cwd".into()),
        (sandbox.to_path_buf(), "/workspace".into()),
        (sandbox.join(".home"), "/userhome".into()),
        (sandbox.join(".moon"), "/moon".into()),
        (sandbox.join(".proto"), "/proto".into()),
    ]);

    // Folders must exists for WASM to compile correctly!
    fs::create_dir_all(sandbox.join(".home")).unwrap();
    fs::create_dir_all(sandbox.join(".moon")).unwrap();
    fs::create_dir_all(sandbox.join(".proto")).unwrap();

    let wasm_file = test_utils::find_wasm_file();
    let mut log_file = wasm_file.clone();
    log_file.set_extension("log");

    let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);
    manifest.timeout_ms = None;
    manifest = manifest.with_allowed_host("*");
    manifest = manifest.with_allowed_paths(virtual_paths.clone().into_iter());

    inject_default_manifest_config(&id, &sandbox.join(".home"), &mut manifest).unwrap();

    manifest.config.extend(config);

    let funcs = create_host_functions(HostData {
        http_client: loader.get_client().unwrap().clone(),
        virtual_paths,
        working_dir: sandbox.to_path_buf(),
    });

    // Remove the file otherwise it keeps growing
    if log_file.exists() {
        let _ = fs::remove_file(&log_file);
    }

    // TODO redo
    if env::var("CI").is_err() {
        let _ = extism::set_log_callback(
            move |line| {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file)
                    .unwrap();

                file.write_all(line.as_bytes()).unwrap();
            },
            "trace",
        );
    }

    PluginContainer::new(id, manifest, funcs).unwrap()
}

pub fn create_extension_with_config(
    id: &str,
    sandbox: &Path,
    config: HashMap<String, String>,
) -> ExtensionTestWrapper {
    ExtensionTestWrapper {
        plugin: create_plugin_container_with_config(id, sandbox, config),
    }
}

pub fn create_extension(id: &str, sandbox: &Path) -> ExtensionTestWrapper {
    create_extension_with_config(id, sandbox, HashMap::new())
}

pub fn create_config_entry<T: Serialize>(key: &str, value: T) -> (String, String) {
    (key.into(), serde_json::to_string(&value).unwrap())
}

pub fn map_config_environment(os: HostOS, arch: HostArch) -> (String, String) {
    create_config_entry(
        "host_environment",
        HostEnvironment {
            arch,
            os,
            ..HostEnvironment::default()
        },
    )
}

pub fn map_config_extension(config: BTreeMap<String, serde_json::Value>) -> (String, String) {
    create_config_entry("moon_extension_config", config)
}

pub fn map_config_id(id: &str) -> (String, String) {
    ("plugin_id".into(), id.to_owned())
}
