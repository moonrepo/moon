// tsconfig.json

use cached::proc_macro::cached;
use moon_error::{map_io_to_fs_error, map_json_to_error, MoonError};
use moon_lang::config_cache;
use moon_utils::{
    fs::{self, sync::read_json},
    json,
    path::standardize_separators,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
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
    pub extends: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_acquisition: Option<TypeAcquisition>,

    // Unknown fields
    #[serde(flatten)]
    pub unknown_fields: BTreeMap<String, Value>,

    // Non-standard
    #[serde(skip)]
    pub dirty: bool,

    #[serde(skip)]
    pub path: PathBuf,
}

impl TsConfigJson {
    pub async fn load_with_extends<T: AsRef<Path>>(path: T) -> Result<TsConfigJson, MoonError> {
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
            path = format!("{}/{}", path, tsconfig_name)
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

        self.dirty = true;
        self.references = Some(references);

        true
    }

    pub fn update_compiler_options(&mut self) -> &mut CompilerOptions {
        if self.compiler_options.is_none() {
            self.compiler_options = Some(CompilerOptions::default());
        }

        self.dirty = true;
        self.compiler_options.as_mut().unwrap()
    }

    pub fn save(&mut self) -> Result<(), MoonError> {
        if self.dirty {
            write_preserved_json(&self.path, self)?;
            self.dirty = false;

            TsConfigJson::write(self.clone())?;
        }

        Ok(())
    }
}

fn merge(a: &mut Value, b: Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            if let Value::Null = a {
                *a = b;
            }
        }
    }
}

pub fn load_to_value<T: AsRef<Path>>(path: T, extend: bool) -> Result<Value, MoonError> {
    let path = path.as_ref();
    let json =
        std::fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    let mut json: Value = serde_json::from_str(&fs::clean_json(json)?)
        .map_err(|e| map_json_to_error(e, path.to_path_buf()))?;

    if extend {
        if let Value::String(s) = &json["extends"] {
            let extends_path = path.parent().unwrap_or_else(|| Path::new("")).join(s);
            let extends_value = load_to_value(&extends_path, extend)?;

            merge(&mut json, extends_value);
        }
    }

    Ok(json)
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
    pub imports_not_used_as_values: Option<String>,

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
    pub module_resolution: Option<ModuleResolution>,

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
    pub plugins: Option<Vec<BTreeMap<String, Value>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_const_enums: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_symlinks: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_value_imports: Option<bool>,

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
    pub watch_options: Option<WatchOptions>,

    // Deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub charset: Option<String>,

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
    #[deprecated]
    pub suppress_excess_property_errors: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated]
    pub suppress_implicit_any_index_errors: Option<bool>,
}

impl CompilerOptions {
    pub fn update_paths(&mut self) -> &mut CompilerOptionsPaths {
        if self.paths.is_none() {
            self.paths = Some(BTreeMap::new());
        }

        self.paths.as_mut().unwrap()
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

// https://www.typescriptlang.org/tsconfig#moduleResolution
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleResolution {
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
    let contents = fs::sync::read_json_string(path)?;
    let mut data = json::parse(&contents).expect("Unable to parse tsconfig.json");

    // We only need to set fields that we modify within Moon,
    // otherwise it's a ton of overhead and maintenance!
    if let Some(references) = &tsconfig.references {
        let mut list = json::JsonValue::new_array();

        for reference in references {
            let mut item = json::JsonValue::new_object();
            item["path"] = json::from(reference.path.clone());

            if let Some(prepend) = reference.prepend {
                item["prepend"] = json::from(prepend);
            }

            list.push(item).unwrap();
        }

        data["references"] = list;
    }

    if let Some(options) = &tsconfig.compiler_options {
        if (options.out_dir.is_some() || options.paths.is_some())
            && data["compilerOptions"].is_empty()
        {
            data["compilerOptions"] = json::JsonValue::new_object();
        }

        if let Some(out_dir) = &options.out_dir {
            data["compilerOptions"]["outDir"] = json::from(out_dir.to_owned());
        }

        if let Some(paths) = &options.paths {
            data["compilerOptions"]["paths"] = json::from(paths.to_owned());
        }
    }

    json::write(path, data, true)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use moon_utils::string_vec;
    use moon_utils::test::get_fixtures_dir;

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

        let mut value1: Value = serde_json::from_str(json_1).unwrap();
        let value2: Value = serde_json::from_str(json_2).unwrap();

        merge(&mut value1, value2);

        let value: TsConfigJson = serde_json::from_value(value1).unwrap();

        assert_eq!(
            value.clone().compiler_options.unwrap().jsx,
            Some(Jsx::React)
        );
        assert_eq!(value.clone().compiler_options.unwrap().no_emit, Some(true));
        assert_eq!(value.compiler_options.unwrap().remove_comments, Some(true));
    }

    #[tokio::test]
    async fn parse_basic_file() {
        let path = get_fixtures_dir("base/tsconfig-json");
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

    #[tokio::test]
    async fn parse_inheriting_file() {
        let path = get_fixtures_dir("base/tsconfig-json/tsconfig.inherits.json");
        let config = TsConfigJson::load_with_extends(&path).await.unwrap();

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

    #[tokio::test]
    async fn parse_inheritance_chain() {
        let path = get_fixtures_dir("base/tsconfig-json/a/tsconfig.json");
        let config = TsConfigJson::load_with_extends(&path).await.unwrap();

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
}
