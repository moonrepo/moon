// deno.json

use cached::proc_macro::cached;
use moon_lang::config_cache;
use serde::{Deserialize, Serialize};
use starbase_utils::json::{self, JsonValue, read_file as read_json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use typescript_tsconfig_json::CompilerOptions;

config_cache!(DenoJson, "deno.json", read_json, write_preserved_json);

// This isn't everything, just what we care about
// https://deno.land/x/deno/cli/schemas/config-file.v1.json
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_options: Option<CompilerOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<DenoJsonLock>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_map: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<BTreeMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<BTreeMap<String, BTreeMap<String, String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<DenoJsonWorkspace>,

    // Non-standard
    #[serde(skip)]
    pub dirty: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DenoJsonWorkspace {
    Members(Vec<String>),
    Config { members: Vec<String> },
}

impl DenoJsonWorkspace {
    pub fn get_members(&self) -> &[String] {
        match self {
            Self::Members(members) => members,
            Self::Config { members } => members,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DenoJsonLock {
    Enabled(bool),
    Path(String),
    Config {
        path: String,
        #[serde(default)]
        frozen: bool,
    },
}

impl DenoJson {
    pub fn save(&mut self) -> miette::Result<()> {
        if !self.dirty.is_empty() {
            write_preserved_json(&self.path, self)?;
            self.dirty.clear();

            DenoJson::write(self.clone())?;
        }

        Ok(())
    }
}

#[track_caller]
fn write_preserved_json(path: &Path, _config: &DenoJson) -> miette::Result<()> {
    let data: JsonValue = json::read_file(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    // for field in &package.dirty {}

    json::write_file_with_config(path, &data, true)?;

    Ok(())
}
