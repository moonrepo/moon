// tsconfig.json

use crate::errors::ConfigError;
use json_comments::StripComments;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{to_string_pretty, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

// This implementation is forked from the wonderful crate "tsconfig", as we need full control for
// integration with the rest of the crates. We also can't wait for upsteam for new updates.
// https://github.com/drivasperez/tsconfig

// Original license: Copyright 2021 Daniel Rivas Perez

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TsConfigJson {
    pub exclude: Option<Vec<String>>,
    pub extends: Option<String>,
    pub files: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub references: Option<Vec<Reference>>,
    pub type_acquisition: Option<TypeAcquisition>,
    pub compile_on_save: Option<bool>,
    pub compiler_options: Option<CompilerOptions>,

    // Unknown fields
    #[serde(flatten)]
    pub unknown_fields: BTreeMap<String, Value>,

    // Non-standard
    #[serde(skip)]
    pub path: PathBuf,
}

impl TsConfigJson {
    pub fn load(path: &Path) -> Result<TsConfigJson, ConfigError> {
        let values = load_to_value(path, false)?;
        let mut cfg: TsConfigJson = serde_json::from_value(values)?;
        cfg.path = path.to_path_buf();

        Ok(cfg)
    }

    pub fn load_with_extends(path: &Path) -> Result<TsConfigJson, ConfigError> {
        let values = load_to_value(path, true)?;
        let mut cfg: TsConfigJson = serde_json::from_value(values)?;
        cfg.path = path.to_path_buf();

        Ok(cfg)
    }

    pub fn add_project_ref(&mut self, path: String) -> bool {
        let mut references = match &self.references {
            Some(refs) => refs,
            None => vec![],
        };

        // Check if the reference already exists
        if references.iter().find(|r| r.path == path).is_some() {
            return false;
        }

        // Add and sort the references
        references.push(Reference {
            path,
            prepend: None,
        });

        references.sort_by_key(|r| r.path);

        self.references = Some(references);

        true
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        fs::write(&self.path, to_string_pretty(self)?)?;

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

pub fn load_to_value(path: &Path, extend: bool) -> Result<Value, ConfigError> {
    let json = std::fs::read_to_string(path)?;
    let mut value = parse_to_value(&json)?;

    if extend {
        if let Value::String(s) = &value["extends"] {
            let extends_path = path.parent().unwrap_or_else(|| Path::new("")).join(s);
            let extends_value = load_to_value(&extends_path, extend)?;

            merge(&mut value, extends_value);
        }
    }

    Ok(value)
}

pub fn parse_to_value(json: &str) -> Result<Value, ConfigError> {
    let mut stripped = String::with_capacity(json.len());
    StripComments::new(json.as_bytes()).read_to_string(&mut stripped)?;

    // Remove trailing commas from objects.
    let pattern = Regex::new(r",(?P<valid>\s*})").unwrap();
    let stripped = pattern.replace_all(&stripped, "$valid");

    let value: Value = serde_json::from_str(&stripped)?;

    Ok(value)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Reference {
    pub path: String,
    pub prepend: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAcquisition {
    pub enable: bool,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub disable_filename_based_type_acquisition: Option<bool>,
}

// https://www.typescriptlang.org/tsconfig#compilerOptions
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
    pub allow_js: Option<bool>,
    pub allow_synthetic_default_imports: Option<bool>,
    pub allow_umd_global_access: Option<bool>,
    pub allow_unreachable_code: Option<bool>,
    pub allow_unused_labels: Option<bool>,
    pub always_strict: Option<bool>,
    pub assume_changes_only_affect_direct_dependencies: Option<bool>,
    pub base_url: Option<String>,
    pub check_js: Option<bool>,
    pub composite: Option<bool>,
    pub declaration_dir: Option<String>,
    pub declaration_map: Option<bool>,
    pub declaration: Option<bool>,
    pub diagnostics: Option<bool>,
    pub disable_referenced_project_load: Option<bool>,
    pub disable_size_limit: Option<bool>,
    pub disable_solution_searching: Option<bool>,
    pub disable_source_of_project_reference_redirect: Option<bool>,
    pub downlevel_iteration: Option<bool>,
    #[serde(rename = "emitBOM")]
    pub emit_bom: Option<bool>,
    pub emit_declaration_only: Option<bool>,
    pub emit_decorator_metadata: Option<bool>,
    pub es_module_interop: Option<bool>,
    pub exact_optional_property_types: Option<bool>,
    pub experimental_decorators: Option<bool>,
    pub explain_files: Option<bool>,
    pub extended_diagnostics: Option<bool>,
    pub force_consistent_casing_in_file_names: Option<bool>,
    pub generate_cpu_profile: Option<bool>,
    pub import_helpers: Option<bool>,
    pub imports_not_used_as_values: Option<String>,
    pub incremental: Option<bool>,
    pub inline_source_map: Option<bool>,
    pub inline_sources: Option<bool>,
    pub isolated_modules: Option<bool>,
    pub jsx_factory: Option<String>,
    pub jsx_fragment_factory: Option<String>,
    pub jsx_import_source: Option<String>,
    pub jsx: Option<Jsx>,
    pub lib: Option<Vec<Lib>>,
    pub list_emitted_files: Option<bool>,
    pub list_files: Option<bool>,
    pub map_root: Option<String>,
    pub max_node_module_js_depth: Option<u32>,
    pub module_resolution: Option<ModuleResolution>,
    pub module: Option<Module>,
    pub new_line: Option<String>,
    pub no_emit_helpers: Option<bool>,
    pub no_emit_on_error: Option<bool>,
    pub no_emit: Option<bool>,
    pub no_error_truncation: Option<bool>,
    pub no_fallthrough_cases_in_switch: Option<bool>,
    pub no_implicit_any: Option<bool>,
    pub no_implicit_override: Option<bool>,
    pub no_implicit_returns: Option<bool>,
    pub no_implicit_this: Option<bool>,
    pub no_lib: Option<bool>,
    pub no_property_access_from_index_signature: Option<bool>,
    pub no_resolve: Option<bool>,
    pub no_unchecked_indexed_access: Option<bool>,
    pub no_unused_locals: Option<bool>,
    pub no_unused_parameters: Option<bool>,
    pub out_dir: Option<String>,
    pub out_file: Option<String>,
    pub paths: Option<BTreeMap<String, Vec<String>>>,
    pub plugins: Option<Vec<BTreeMap<String, Value>>>,
    pub preserve_const_enums: Option<bool>,
    pub preserve_symlinks: Option<bool>,
    pub preserve_value_imports: Option<bool>,
    pub preserve_watch_output: Option<bool>,
    pub pretty: Option<bool>,
    pub react_namespace: Option<String>,
    pub remove_comments: Option<bool>,
    pub resolve_json_module: Option<bool>,
    pub root_dir: Option<String>,
    pub root_dirs: Option<Vec<String>>,
    pub skip_default_lib_check: Option<bool>,
    pub skip_lib_check: Option<bool>,
    pub source_map: Option<bool>,
    pub source_root: Option<String>,
    pub strict_bind_call_apply: Option<bool>,
    pub strict_function_types: Option<bool>,
    pub strict_null_checks: Option<bool>,
    pub strict_property_initialization: Option<bool>,
    pub strict: Option<bool>,
    pub strip_internal: Option<bool>,
    pub target: Option<Target>,
    pub trace_resolution: Option<bool>,
    pub ts_build_info_file: Option<String>,
    pub type_roots: Option<Vec<String>>,
    pub types: Option<Vec<String>>,
    pub use_define_for_class_fields: Option<bool>,
    pub use_unknown_in_catch_variables: Option<bool>,
    pub watch_options: Option<WatchOptions>,

    #[deprecated]
    pub charset: Option<String>,
    #[deprecated]
    pub keyof_strings_only: Option<bool>,
    #[deprecated]
    pub no_implicit_use_strict: Option<bool>,
    #[deprecated]
    pub no_strict_generic_checks: Option<bool>,
    #[deprecated]
    pub out: Option<String>,
    #[deprecated]
    pub suppress_excess_property_errors: Option<bool>,
    #[deprecated]
    pub suppress_implicit_any_index_errors: Option<bool>,
}

// https://www.typescriptlang.org/tsconfig#watch-options
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct WatchOptions {
    pub exclude_directories: Option<Vec<String>>,
    pub exclude_files: Option<Vec<String>>,
    pub fallback_polling: Option<String>,
    pub synchronous_watch_directory: Option<bool>,
    pub watch_directory: Option<String>,
    pub watch_file: Option<String>,
}

// https://www.typescriptlang.org/tsconfig#module
#[derive(Clone, Debug, PartialEq, Serialize)]
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
            "ES2022" => Module::Es2020,
            "ESNEXT" => Module::EsNext,
            "NODE12" => Module::Node12,
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
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleResolution {
    Classic,
    Node,
    Node12,
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
            "NODENEXT" => ModuleResolution::NodeNext,
            _ => ModuleResolution::Node,
        };

        Ok(r)
    }
}

// https://www.typescriptlang.org/tsconfig#jsx
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Jsx {
    React,
    ReactJsx,
    ReactJsxdev,
    ReactNative,
    Preserve,
}

// https://www.typescriptlang.org/tsconfig#target
#[derive(Clone, Debug, PartialEq, Serialize)]
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
            "ESNEXT" => Target::EsNext,
            other => Target::Other(other.to_string()),
        };

        Ok(d)
    }
}

// https://www.typescriptlang.org/tsconfig#lib
#[derive(Clone, Debug, PartialEq)]
pub enum Lib {
    Dom,
    DomIterable,
    Es5,
    Es6,
    Es7,
    Es2015,
    Es2015Core,
    Es2015Collection,
    Es2015Generator,
    Es2015Iterable,
    Es2015Promise,
    Es2015Proxy,
    Es2015Reflect,
    Es2015Symbol,
    Es2015SymbolWellKnown,
    Es2016,
    Es2016ArrayInclude,
    Es2017,
    Es2017Intl,
    Es2017Object,
    Es2017SharedMemory,
    Es2017String,
    Es2017TypedArrays,
    Es2018,
    Es2018Intl,
    Es2018Promise,
    Es2018RegExp,
    Es2019,
    Es2019Array,
    Es2019Object,
    Es2019String,
    Es2019Symbol,
    Es2020,
    Es2020String,
    Es2020SymbolWellknown,
    Es2021,
    Es2021Promise,
    Es2021String,
    Es2021Weakref,
    EsNext,
    EsNextArray,
    EsNextAsyncIterable,
    EsNextIntl,
    EsNextSymbol,
    ScriptHost,
    WebWorker,
    Other(String),
}

impl fmt::Display for Lib {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'de> Deserialize<'de> for Lib {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.to_uppercase();

        let d = match s.as_str() {
            "DOM.ITERABLE" => Lib::DomIterable,
            "DOM" => Lib::Dom,
            "ES5" => Lib::Es5,
            "ES6" => Lib::Es6,
            "ES7" => Lib::Es7,
            "ES2015.CORE" => Lib::Es2015Core,
            "ES2015.COLLECTION" => Lib::Es2015Collection,
            "ES2015.GENERATOR" => Lib::Es2015Generator,
            "ES2015.ITERABLE" => Lib::Es2015Iterable,
            "ES2015.PROMISE" => Lib::Es2015Promise,
            "ES2015.PROXY" => Lib::Es2015Proxy,
            "ES2015.REFLECT" => Lib::Es2015Reflect,
            "ES2015.SYMBOL.WELLKNOWN" => Lib::Es2015SymbolWellKnown,
            "ES2015.SYMBOL" => Lib::Es2015Symbol,
            "ES2015" => Lib::Es2015,
            "ES2016.ARRAY.INCLUDE" => Lib::Es2016ArrayInclude,
            "ES2016" => Lib::Es2016,
            "ES2017.INTL" => Lib::Es2017Intl,
            "ES2017.OBJECT" => Lib::Es2017Object,
            "ES2017.SHAREDMEMORY" => Lib::Es2017SharedMemory,
            "ES2017.STRING" => Lib::Es2017String,
            "ES2017.TYPEDARRAYS" => Lib::Es2017TypedArrays,
            "ES2017" => Lib::Es2017,
            "ES2018.INTL" => Lib::Es2018Intl,
            "ES2018.PROMISE" => Lib::Es2018Promise,
            "ES2018.REGEXP" => Lib::Es2018RegExp,
            "ES2018" => Lib::Es2018,
            "ES2019.ARRAY" => Lib::Es2019Array,
            "ES2019.OBJECT" => Lib::Es2019Object,
            "ES2019.STRING" => Lib::Es2019String,
            "ES2019.SYMBOL" => Lib::Es2019Symbol,
            "ES2019" => Lib::Es2019,
            "ES2020.STRING" => Lib::Es2020String,
            "ES2020.SYMBOL.WELLKNOWN" => Lib::Es2020SymbolWellknown,
            "ES2021.PROMISE" => Lib::Es2021Promise,
            "ES2021.STRING" => Lib::Es2021String,
            "ES2021.WEAKREF" => Lib::Es2021Weakref,
            "ES2021" => Lib::Es2021,
            "ESNEXT.ARRAY" => Lib::EsNextArray,
            "ESNEXT.ASYNCITERABLE" => Lib::EsNextAsyncIterable,
            "ESNEXT.INTL" => Lib::EsNextIntl,
            "ESNEXT.SYMBOL" => Lib::EsNextSymbol,
            "ESNEXT" => Lib::EsNext,
            "SCRIPTHOST" => Lib::ScriptHost,
            "WEBWORKER" => Lib::WebWorker,
            other => Lib::Other(other.to_string()),
        };

        Ok(d)
    }
}

impl Serialize for Lib {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = match self {
            Lib::DomIterable => "DOM.ITERABLE".to_owned(),
            Lib::Es2015Core => "ES2015.CORE".to_owned(),
            Lib::Es2015Collection => "ES2015.COLLECTION".to_owned(),
            Lib::Es2015Generator => "ES2015.GENERATOR".to_owned(),
            Lib::Es2015Iterable => "ES2015.ITERABLE".to_owned(),
            Lib::Es2015Promise => "ES2015.PROMISE".to_owned(),
            Lib::Es2015Proxy => "ES2015.PROXY".to_owned(),
            Lib::Es2015Reflect => "ES2015.REFLECT".to_owned(),
            Lib::Es2015SymbolWellKnown => "ES2015.SYMBOL.WELLKNOWN".to_owned(),
            Lib::Es2015Symbol => "ES2015.SYMBOL".to_owned(),
            Lib::Es2016ArrayInclude => "ES2016.ARRAY.INCLUDE".to_owned(),
            Lib::Es2017Intl => "ES2017.INTL".to_owned(),
            Lib::Es2017Object => "ES2017.OBJECT".to_owned(),
            Lib::Es2017SharedMemory => "ES2017.SHAREDMEMORY".to_owned(),
            Lib::Es2017String => "ES2017.STRING".to_owned(),
            Lib::Es2017TypedArrays => "ES2017.TYPEDARRAYS".to_owned(),
            Lib::Es2018Intl => "ES2018.INTL".to_owned(),
            Lib::Es2018Promise => "ES2018.PROMISE".to_owned(),
            Lib::Es2018RegExp => "ES2018.REGEXP".to_owned(),
            Lib::Es2019Array => "ES2019.ARRAY".to_owned(),
            Lib::Es2019Object => "ES2019.OBJECT".to_owned(),
            Lib::Es2019String => "ES2019.STRING".to_owned(),
            Lib::Es2019Symbol => "ES2019.SYMBOL".to_owned(),
            Lib::Es2020String => "ES2020.STRING".to_owned(),
            Lib::Es2020SymbolWellknown => "ES2020.SYMBOL.WELLKNOWN".to_owned(),
            Lib::Es2021Promise => "ES2021.PROMISE".to_owned(),
            Lib::Es2021String => "ES2021.STRING".to_owned(),
            Lib::Es2021Weakref => "ES2021.WEAKREF".to_owned(),
            Lib::EsNextArray => "ESNEXT.ARRAY".to_owned(),
            Lib::EsNextAsyncIterable => "ESNEXT.ASYNCITERABLE".to_owned(),
            Lib::EsNextIntl => "ESNEXT.INTL".to_owned(),
            Lib::EsNextSymbol => "ESNEXT.SYMBOL".to_owned(),
            other => format!("{:?}", other),
        };

        serializer.serialize_str(value.to_lowercase().as_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use moon_utils::test::get_fixtures_dir;

    #[test]
    fn merge_two_configs() {
        let json_1 = r#"{"compilerOptions": {"jsx": "react", "noEmit": true,}}"#;
        let json_2 = r#"{"compilerOptions": {"jsx": "preserve", "removeComments": true}}"#;

        let mut value1: Value = parse_to_value(json_1).unwrap();
        let value2: Value = parse_to_value(json_2).unwrap();

        merge(&mut value1, value2);

        let value: TsConfigJson = serde_json::from_value(value1).unwrap();

        assert_eq!(
            value.clone().compiler_options.unwrap().jsx,
            Some(Jsx::React)
        );
        assert_eq!(value.clone().compiler_options.unwrap().no_emit, Some(true));
        assert_eq!(value.compiler_options.unwrap().remove_comments, Some(true));
    }

    #[test]
    fn parse_basic_file() {
        let path = get_fixtures_dir("base/tsconfig-json/tsconfig.default.json");
        let config = TsConfigJson::load(&path).unwrap();

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
        let path = get_fixtures_dir("base/tsconfig-json/tsconfig.inherits.json");
        let config = TsConfigJson::load_with_extends(&path).unwrap();

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
        let path = get_fixtures_dir("base/tsconfig-json/a/tsconfig.inherits_again.json");
        let config = TsConfigJson::load_with_extends(&path).unwrap();

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
}
