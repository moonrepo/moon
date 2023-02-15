// deno.json

use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::config_cache;
use moon_utils::json::{self, read as read_json, JsonValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

config_cache!(DenoJson, "deno.json", read_json, write_preserved_json);

// This isn't everything, just what we care about
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DenoJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<DenoJsonLock>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_map: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<BTreeMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<BTreeMap<String, String>>,

    // Non-standard
    #[serde(skip)]
    pub dirty: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DenoJsonLock {
    Enabled(bool),
    Path(String),
}

impl DenoJson {
    pub fn save(&mut self) -> Result<(), MoonError> {
        if !self.dirty.is_empty() {
            write_preserved_json(&self.path, self)?;
            self.dirty.clear();

            DenoJson::write(self.clone())?;
        }

        Ok(())
    }
}

#[track_caller]
fn write_preserved_json(path: &Path, _config: &DenoJson) -> Result<(), MoonError> {
    let data: JsonValue = json::read(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    // for field in &package.dirty {}

    json::write_with_config(path, data, true)?;

    Ok(())
}
