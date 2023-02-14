// tsconfig.json

use cached::proc_macro::cached;
use moon_error::{map_json_to_error, MoonError};
use moon_lang::config_cache;
use moon_utils::{
    json::{self, read as read_json, JsonMap, JsonValue},
    path::standardize_separators,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

config_cache!(
    TsConfigJson,
    "tsconfig.json",
    read_json,
    write_preserved_json
);

// This implementation is forked from the wonderful crate "tsconfig", as we need full control for
// integration with the rest of the crates. We also can't wait for upsteam for new updates.
// https://github.com/drivasperez/tsconfig

// Original license: Copyright 2021 Daniel Rivas Perez

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TsConfigExtends {
    String(String),
    Array(Vec<String>),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_on_save: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_options: Option<CompilerOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<TsConfigExtends>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_acquisition: Option<TypeAcquisition>,

    // Non-standard
    #[serde(skip)]
    pub dirty: Vec<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl TsConfigJson {
    pub fn load_with_extends<T: AsRef<Path>>(path: T) -> Result<TsConfigJson, MoonError> {
        let path = path.as_ref();
        let values = load_to_value(path, true)?;

        let mut cfg: TsConfigJson =
            serde_json::from_value(values).map_err(|e| map_json_to_error(e, path.to_path_buf()))?;
        cfg.path = path.to_path_buf();

        Ok(cfg)
    }

    /// Add a project reference to the `references` field with the defined
    /// path and tsconfig file name, and sort the list based on path.
    /// Return true if the new value is different from the old value.
    pub fn add_project_ref<T: AsRef<str>>(&mut self, base_path: T, tsconfig_name: T) -> bool {
        let base_path = base_path.as_ref();
        let tsconfig_name = tsconfig_name.as_ref();
        let mut path = standardize_separators(base_path);

        // File name is optional when using standard naming
        if tsconfig_name != "tsconfig.json" {
            path = format!("{path}/{tsconfig_name}")
        };

        let mut references = match &self.references {
            Some(refs) => refs.clone(),
            None => Vec::<Reference>::new(),
        };

        // Check if the reference already exists
        if references
            .iter()
            .any(|r| r.path == path || r.path == base_path)
        {
            return false;
        }

        // Add and sort the references
        references.push(Reference {
            path,
            prepend: None,
        });

        references.sort_by_key(|r| r.path.clone());

        self.dirty.push("references".into());
        self.references = Some(references);

        true
    }

    pub fn update_compiler_options<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut CompilerOptions) -> bool,
    {
        let updated;

        if let Some(options) = self.compiler_options.as_mut() {
            updated = updater(options);
        } else {
            let mut options = CompilerOptions::default();

            updated = updater(&mut options);

            if updated {
                self.compiler_options = Some(options);
            }
        }

        if updated {
            self.dirty.push("compilerOptions".into());
        }

        updated
    }

    pub fn save(&mut self) -> Result<(), MoonError> {
        if !self.dirty.is_empty() {
            write_preserved_json(&self.path, self)?;
            self.dirty.clear();

            TsConfigJson::write(self.clone())?;
        }

        Ok(())
    }
}

pub fn load_to_value<T: AsRef<Path>>(path: T, extend: bool) -> Result<JsonValue, MoonError> {
    let path = path.as_ref();
    let mut merged_file = JsonValue::Object(JsonMap::new());
    let last_file: JsonValue = json::read(path)?;

    if extend {
        let extends_root = path.parent().unwrap_or_else(|| Path::new(""));

        match &last_file["extends"] {
            JsonValue::Array(list) => {
                for item in list {
                    if let JsonValue::String(value) = item {
                        merged_file = json::merge(
                            &merged_file,
                            &load_to_value(extends_root.join(value), extend)?,
                        );
                    }
                }
            }
            JsonValue::String(value) => {
                merged_file = json::merge(
                    &merged_file,
                    &load_to_value(extends_root.join(value), extend)?,
                );
            }
            _ => {}
        }
    }

    merged_file = json::merge(&merged_file, &last_file);

    Ok(merged_file)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reference {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepend: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAcquisition {
    pub enable: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_filename_based_type_acquisition: Option<bool>,
}

pub type CompilerOptionsPaths = BTreeMap<String, Vec<String>>;

// https://www.typescriptlang.org/tsconfig#compilerOptions
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_js: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_arbitrary_extensions: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_importing_ts_extensions: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_synthetic_default_imports: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_umd_global_access: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unreachable_code: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unused_labels: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_strict: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assume_changes_only_affect_direct_dependencies: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_js: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_conditions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_map: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_referenced_project_load: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_size_limit: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_solution_searching: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_source_of_project_reference_redirect: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub downlevel_iteration: Option<bool>,

    #[serde(rename = "emitBOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_bom: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_declaration_only: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_decorator_metadata: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub es_module_interop: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_optional_property_types: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental_decorators: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain_files: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_diagnostics: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_consistent_casing_in_file_names: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_cpu_profile: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_helpers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub incremental: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_source_map: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_sources: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolated_modules: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx_factory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx_fragment_factory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx_import_source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx: Option<Jsx>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_emitted_files: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_files: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_root: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_node_module_js_depth: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<Module>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_detection: Option<ModuleDetection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_resolution: Option<ModuleResolution>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_suffixes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_emit_helpers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_emit_on_error: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_emit: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_error_truncation: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallthrough_cases_in_switch: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_implicit_any: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_implicit_override: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_implicit_returns: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_implicit_this: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_lib: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_property_access_from_index_signature: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_resolve: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_unchecked_indexed_access: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_unused_locals: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_unused_parameters: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_file: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<CompilerOptionsPaths>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<BTreeMap<String, JsonValue>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_const_enums: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_symlinks: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_watch_output: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretty: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub react_namespace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_comments: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_json_module: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_package_json_exports: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_package_json_imports: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dirs: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_default_lib_check: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_lib_check: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_bind_call_apply: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_function_types: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_null_checks: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_property_initialization: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_internal: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_resolution: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_build_info_file: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_roots: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_define_for_class_fields: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_unknown_in_catch_variables: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim_module_syntax: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_options: Option<WatchOptions>,

    // Deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub charset: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports_not_used_as_values: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub keyof_strings_only: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub no_implicit_use_strict: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub no_strict_generic_checks: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub out: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_value_imports: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub suppress_excess_property_errors: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub suppress_implicit_any_index_errors: Option<bool>,
}

impl CompilerOptions {
    pub fn update_paths(&mut self, paths: CompilerOptionsPaths) -> bool {
        let mut updated = false;

        if let Some(current_paths) = self.paths.as_mut() {
            for (path, mut patterns) in paths {
                if let Some(current_patterns) = current_paths.get_mut(&path) {
                    patterns.sort();
                    current_patterns.sort();

                    if &patterns != current_patterns {
                        updated = true;
                        current_paths.insert(path, patterns);
                    }
                } else {
                    updated = true;
                    current_paths.insert(path, patterns);
                }
            }
        } else {
            updated = true;
            self.paths = Some(paths);
        }

        updated
    }
}

// https://www.typescriptlang.org/tsconfig#watch-options
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_directories: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_polling: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub synchronous_watch_directory: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_directory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_file: Option<String>,
}

// https://www.typescriptlang.org/tsconfig#module
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Module {
    Amd,
    CommonJs,
    Es6,
    Es2015,
    Es2020,
    Es2022,
    EsNext,
    Node12,
    Node16,
    NodeNext,
    None,
    System,
    Umd,
    Other(String),
}

impl<'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.to_uppercase();

        let r = match s.as_str() {
            "AMD" => Module::Amd,
            "COMMONJS" => Module::CommonJs,
            "ES6" => Module::Es6,
            "ES2015" => Module::Es2015,
            "ES2020" => Module::Es2020,
            "ES2022" => Module::Es2022,
            "ESNEXT" => Module::EsNext,
            "NODE12" => Module::Node12,
            "NODE16" => Module::Node16,
            "NODENEXT" => Module::NodeNext,
            "NONE" => Module::None,
            "SYSTEM" => Module::System,
            "UMD" => Module::Umd,
            other => Module::Other(other.to_string()),
        };

        Ok(r)
    }
}

// https://www.typescriptlang.org/tsconfig#moduleDetection
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleDetection {
    Auto,
    Legacy,
    Force,
}

impl<'de> Deserialize<'de> for ModuleDetection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.to_uppercase();

        let r = match s.as_str() {
            "LEGACY" => ModuleDetection::Legacy,
            "FORCE" => ModuleDetection::Force,
            _ => ModuleDetection::Auto,
        };

        Ok(r)
    }
}

// https://www.typescriptlang.org/tsconfig#moduleResolution
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleResolution {
    Bundler,
    Classic,
    Node,
    Node12,
    Node16,
    NodeNext,
}

impl<'de> Deserialize<'de> for ModuleResolution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.to_uppercase();

        let r = match s.as_str() {
            "BUNDLER" => ModuleResolution::Bundler,
            "CLASSIC" => ModuleResolution::Classic,
            "NODE12" => ModuleResolution::Node12,
            "NODE16" => ModuleResolution::Node16,
            "NODENEXT" => ModuleResolution::NodeNext,
            _ => ModuleResolution::Node,
        };

        Ok(r)
    }
}

// https://www.typescriptlang.org/tsconfig#jsx
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Jsx {
    React,
    ReactJsx,
    ReactJsxdev,
    ReactNative,
    Preserve,
}

// https://www.typescriptlang.org/tsconfig#target
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Es3,
    Es5,
    Es6,
    Es7,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    EsNext,
    Other(String),
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.to_uppercase();

        let d = match s.as_str() {
            "ES3" => Target::Es3,
            "ES5" => Target::Es5,
            "ES6" => Target::Es6,
            "ES7" => Target::Es7,
            "ES2015" => Target::Es2015,
            "ES2016" => Target::Es2016,
            "ES2017" => Target::Es2017,
            "ES2018" => Target::Es2018,
            "ES2019" => Target::Es2019,
            "ES2020" => Target::Es2020,
            "ES2021" => Target::Es2021,
            "ES2022" => Target::Es2022,
            "ESNEXT" => Target::EsNext,
            other => Target::Other(other.to_string()),
        };

        Ok(d)
    }
}

// https://github.com/serde-rs/json/issues/858
// `serde-json` does NOT preserve original order when serializing the struct,
// so we need to hack around this by using the `json` crate and manually
// making the changes. For this to work correctly, we need to read the json
// file again and parse it with `json`, then stringify it with `json`.
#[track_caller]
fn write_preserved_json(path: &Path, tsconfig: &TsConfigJson) -> Result<(), MoonError> {
    let mut data: JsonValue = json::read(path)?;

    // We only need to set fields that we modify within moon,
    // otherwise it's a ton of overhead and maintenance!
    for field in &tsconfig.dirty {
        match field.as_ref() {
            "references" => {
                if let Some(references) = &tsconfig.references {
                    let mut list = vec![];

                    for reference in references {
                        let mut item = json::json!({});
                        item["path"] = JsonValue::from(reference.path.clone());

                        if let Some(prepend) = reference.prepend {
                            item["prepend"] = JsonValue::from(prepend);
                        }

                        list.push(item);
                    }

                    data[field] = JsonValue::Array(list);
                }
            }
            "compilerOptions" => {
                if let Some(options) = &tsconfig.compiler_options {
                    if (options.out_dir.is_some() || options.paths.is_some())
                        && !data[field].is_object()
                    {
                        data[field] = json::json!({});
                    }

                    if let Some(out_dir) = &options.out_dir {
                        data[field]["outDir"] = JsonValue::from(out_dir.to_owned());
                    }

                    if let Some(paths) = &options.paths {
                        data[field]["paths"] = JsonValue::from_iter(paths.to_owned());
                    }
                }
            }
            _ => {}
        }
    }

    json::write_with_config(path, data, true)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use moon_test_utils::{assert_fs::prelude::*, create_temp_dir, get_fixtures_path};
    use moon_utils::string_vec;

    #[test]
    fn preserves_when_saving() {
        let json = "{\n  \"compilerOptions\": {},\n  \"files\": [\n    \"**/*\"\n  ]\n}\n";

        let dir = create_temp_dir();
        let file = dir.child("tsconfig.json");
        file.write_str(json).unwrap();

        let mut package = TsConfigJson::read(dir.path()).unwrap().unwrap();

        // Trigger dirty
        package.dirty.push("unknown".into());

        package.save().unwrap();

        assert_eq!(json::read_to_string(file.path()).unwrap(), json);
    }

    #[test]
    fn serializes_special_fields() {
        let actual = TsConfigJson {
            compiler_options: Some(CompilerOptions {
                module: Some(Module::EsNext),
                module_resolution: Some(ModuleResolution::Node12),
                jsx: Some(Jsx::ReactJsxdev),
                target: Some(Target::Es6),
                lib: Some(string_vec![
                    "dom",
                    "es2015.generator",
                    "es2016.array.include",
                    "es2017.sharedmemory",
                    "es2018.intl",
                    "es2019.symbol",
                    "es2020.symbol.wellknown",
                    "es2021.weakref",
                ]),
                ..CompilerOptions::default()
            }),
            ..TsConfigJson::default()
        };

        let expected = serde_json::json!({
            "compilerOptions": {
                "jsx": "react-jsxdev",
                "lib": [
                    "dom",
                    "es2015.generator",
                    "es2016.array.include",
                    "es2017.sharedmemory",
                    "es2018.intl",
                    "es2019.symbol",
                    "es2020.symbol.wellknown",
                    "es2021.weakref",
                ],
                "module": "esnext",
                "moduleResolution": "node12",
                "target": "es6",
            },
        });

        assert_eq!(
            serde_json::to_string(&actual).unwrap(),
            serde_json::to_string(&expected).unwrap(),
        );
    }

    #[test]
    fn deserializes_special_fields() {
        let actual = serde_json::json!({
            "compilerOptions": {
                "jsx": "react-native",
                "lib": [
                    "dom",
                    "es2015.collection",
                    "es2016",
                    "es2017.typedarrays",
                    "es2018.promise",
                    "es2019.string",
                    "es2020",
                    "es2021.weakref",
                ],
                "module": "es2015",
                "moduleResolution": "classic",
                "target": "esnext",
            },
        });

        let expected = TsConfigJson {
            compiler_options: Some(CompilerOptions {
                jsx: Some(Jsx::ReactNative),
                lib: Some(string_vec![
                    "dom",
                    "es2015.collection",
                    "es2016",
                    "es2017.typedarrays",
                    "es2018.promise",
                    "es2019.string",
                    "es2020",
                    "es2021.weakref",
                ]),
                module: Some(Module::Es2015),
                module_resolution: Some(ModuleResolution::Classic),
                target: Some(Target::EsNext),
                ..CompilerOptions::default()
            }),
            ..TsConfigJson::default()
        };

        let actual_typed: TsConfigJson = serde_json::from_value(actual).unwrap();

        assert_eq!(actual_typed, expected);
    }

    #[test]
    fn merge_two_configs() {
        let json_1 = r#"{"compilerOptions": {"jsx": "react", "noEmit": true}}"#;
        let json_2 = r#"{"compilerOptions": {"jsx": "preserve", "removeComments": true}}"#;

        let value1: JsonValue = serde_json::from_str(json_1).unwrap();
        let value2: JsonValue = serde_json::from_str(json_2).unwrap();

        let new_value = json::merge(&value1, &value2);
        let config: TsConfigJson = serde_json::from_value(new_value).unwrap();

        assert_eq!(
            config.clone().compiler_options.unwrap().jsx,
            Some(Jsx::Preserve)
        );
        assert_eq!(config.clone().compiler_options.unwrap().no_emit, Some(true));
        assert_eq!(config.compiler_options.unwrap().remove_comments, Some(true));
    }

    #[test]
    fn parse_basic_file() {
        let path = get_fixtures_path("base/tsconfig-json");
        let config = TsConfigJson::read_with_name(path, "tsconfig.default.json")
            .unwrap()
            .unwrap();

        assert_eq!(
            config.compiler_options.clone().unwrap().target,
            Some(Target::Es5)
        );
        assert_eq!(
            config.compiler_options.clone().unwrap().module,
            Some(Module::CommonJs)
        );
        assert_eq!(config.compiler_options.unwrap().strict, Some(true));
    }

    #[test]
    fn parse_inheriting_file() {
        let path = get_fixtures_path("base/tsconfig-json/tsconfig.inherits.json");
        let config = TsConfigJson::load_with_extends(path).unwrap();

        assert_eq!(
            config
                .compiler_options
                .clone()
                .unwrap()
                .use_define_for_class_fields,
            Some(false)
        );

        assert_eq!(
            config.compiler_options.clone().unwrap().declaration,
            Some(true)
        );

        assert_eq!(
            config.compiler_options.unwrap().trace_resolution,
            Some(false)
        );
    }

    #[test]
    fn parse_inheritance_chain() {
        let path = get_fixtures_path("base/tsconfig-json/a/tsconfig.json");
        let config = TsConfigJson::load_with_extends(path).unwrap();

        assert_eq!(
            config
                .compiler_options
                .clone()
                .unwrap()
                .use_define_for_class_fields,
            Some(false)
        );

        assert_eq!(
            config.compiler_options.clone().unwrap().declaration,
            Some(true)
        );

        assert_eq!(
            config.compiler_options.clone().unwrap().trace_resolution,
            Some(false)
        );

        assert_eq!(config.compiler_options.unwrap().jsx, Some(Jsx::ReactNative));
    }

    #[test]
    fn parse_multi_inheritance_chain() {
        let path = get_fixtures_path("base/tsconfig-json/tsconfig.multi-inherits.json");
        let config = TsConfigJson::load_with_extends(path).unwrap();

        let options = config.compiler_options.as_ref().unwrap();

        assert_eq!(options.declaration, Some(true));
        assert_eq!(options.module_resolution, Some(ModuleResolution::Bundler));
        assert_eq!(options.module, Some(Module::EsNext));
        assert_eq!(options.jsx, Some(Jsx::Preserve));
        assert_eq!(options.trace_resolution, Some(false));
    }

    mod add_project_ref {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut tsc = TsConfigJson::default();

            assert_eq!(tsc.references, None);

            assert!(tsc.add_project_ref("../sibling", "tsconfig.json"));

            assert_eq!(
                tsc.references.unwrap(),
                vec![Reference {
                    path: "../sibling".to_owned(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn doesnt_add_if_set() {
            let mut tsc = TsConfigJson {
                references: Some(vec![Reference {
                    path: "../sibling".to_owned(),
                    prepend: None,
                }]),
                ..TsConfigJson::default()
            };

            assert!(!tsc.add_project_ref("../sibling", "tsconfig.json"));

            assert_eq!(
                tsc.references.unwrap(),
                vec![Reference {
                    path: "../sibling".to_owned(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn includes_custom_config_name() {
            let mut tsc = TsConfigJson::default();

            assert_eq!(tsc.references, None);

            assert!(tsc.add_project_ref("../sibling", "tsconfig.ref.json"));

            assert_eq!(
                tsc.references.unwrap(),
                vec![Reference {
                    path: "../sibling/tsconfig.ref.json".to_owned(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn forces_forward_slash() {
            let mut tsc = TsConfigJson::default();

            assert_eq!(tsc.references, None);

            assert!(tsc.add_project_ref("..\\sibling", "tsconfig.json"));

            assert_eq!(
                tsc.references.unwrap(),
                vec![Reference {
                    path: "../sibling".to_owned(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn appends_and_sorts_list() {
            let mut tsc = TsConfigJson {
                references: Some(vec![Reference {
                    path: "../sister".to_owned(),
                    prepend: None,
                }]),
                ..TsConfigJson::default()
            };

            assert!(tsc.add_project_ref("../brother", "tsconfig.json"));

            assert_eq!(
                tsc.references.unwrap(),
                vec![
                    Reference {
                        path: "../brother".to_owned(),
                        prepend: None,
                    },
                    Reference {
                        path: "../sister".to_owned(),
                        prepend: None,
                    }
                ]
            );
        }
    }

    mod update_compiler_options {
        use super::*;

        #[test]
        fn creates_if_none_and_returns_true() {
            let mut tsc = TsConfigJson::default();

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./test".into());
                true
            });

            assert!(updated);
            assert_eq!(
                tsc.compiler_options
                    .as_ref()
                    .unwrap()
                    .out_dir
                    .as_ref()
                    .unwrap(),
                "./test"
            )
        }

        #[test]
        fn doesnt_create_if_none_and_returns_false() {
            let mut tsc = TsConfigJson::default();

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./test".into());
                false
            });

            assert!(!updated);
            assert_eq!(tsc.compiler_options, None)
        }

        #[test]
        fn can_update_existing() {
            let mut tsc = TsConfigJson {
                compiler_options: Some(CompilerOptions {
                    out_dir: Some("./old".into()),
                    ..CompilerOptions::default()
                }),
                ..TsConfigJson::default()
            };

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./new".into());
                true
            });

            assert!(updated);
            assert_eq!(
                tsc.compiler_options
                    .as_ref()
                    .unwrap()
                    .out_dir
                    .as_ref()
                    .unwrap(),
                "./new"
            )
        }

        mod paths {
            use super::*;

            #[test]
            fn sets_if_none() {
                let mut opts = CompilerOptions::default();

                let updated = opts.update_paths(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["index.ts"],
                )]));

                assert!(updated);
                assert_eq!(
                    *opts.paths.as_ref().unwrap().get("alias").unwrap(),
                    string_vec!["index.ts"]
                );
            }

            #[test]
            fn sets_multiple() {
                let mut opts = CompilerOptions::default();

                let updated = opts.update_paths(BTreeMap::from_iter([
                    ("one".into(), string_vec!["one.ts"]),
                    ("two".into(), string_vec!["two.ts"]),
                    ("three".into(), string_vec!["three.ts"]),
                ]));

                assert!(updated);
                assert_eq!(opts.paths.as_ref().unwrap().len(), 3);
            }

            #[test]
            fn overrides_existing_value() {
                let mut opts = CompilerOptions {
                    paths: Some(BTreeMap::from_iter([(
                        "alias".into(),
                        string_vec!["old.ts"],
                    )])),
                    ..CompilerOptions::default()
                };

                let updated = opts.update_paths(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["new.ts"],
                )]));

                assert!(updated);
                assert_eq!(
                    *opts.paths.as_ref().unwrap().get("alias").unwrap(),
                    string_vec!["new.ts"]
                );
            }

            #[test]
            fn doesnt_overrides_same_value() {
                let mut opts = CompilerOptions {
                    paths: Some(BTreeMap::from_iter([(
                        "alias".into(),
                        string_vec!["./src", "./other"],
                    )])),
                    ..CompilerOptions::default()
                };

                let updated = opts.update_paths(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["./src", "./other"],
                )]));

                assert!(!updated);

                let updated = opts.update_paths(BTreeMap::from_iter([(
                    "alias".into(),
                    string_vec!["./other", "./src"],
                )]));

                assert!(!updated);
            }
        }
    }
}
