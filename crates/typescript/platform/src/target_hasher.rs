use moon_config::TypeScriptConfig;
use moon_error::MoonError;
use moon_hasher::{hash_btree, Hasher, Sha256};
use moon_typescript_lang::tsconfig::{CompilerOptions, TsConfigJson};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptTargetHasher {
    // `tsconfig.json` `compilerOptions`
    compiler_options: BTreeMap<String, String>,
}

impl TypeScriptTargetHasher {
    pub fn generate(
        config: &TypeScriptConfig,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<TypeScriptTargetHasher, MoonError> {
        let mut hasher = TypeScriptTargetHasher::default();

        if let Some(root_tsconfig) =
            TsConfigJson::read_with_name(workspace_root, &config.root_config_file_name)?
        {
            if let Some(compiler_options) = &root_tsconfig.compiler_options {
                hasher.hash_compiler_options(compiler_options);
            }
        }

        if let Some(tsconfig) =
            TsConfigJson::read_with_name(project_root, &config.project_config_file_name)?
        {
            if let Some(compiler_options) = &tsconfig.compiler_options {
                hasher.hash_compiler_options(compiler_options);
            }
        }

        Ok(hasher)
    }

    /// Hash compiler options that may alter compiled/generated output.
    pub fn hash_compiler_options(&mut self, compiler_options: &CompilerOptions) {
        if let Some(jsx) = &compiler_options.jsx {
            self.compiler_options
                .insert("jsx".to_owned(), format!("{jsx:?}"));
        }

        if let Some(jsx_factory) = &compiler_options.jsx_factory {
            self.compiler_options
                .insert("jsxFactory".to_owned(), format!("{jsx_factory:?}"));
        }

        if let Some(jsx_fragment_factory) = &compiler_options.jsx_fragment_factory {
            self.compiler_options.insert(
                "jsxFragmentFactory".to_owned(),
                format!("{jsx_fragment_factory:?}"),
            );
        }

        if let Some(jsx_import_source) = &compiler_options.jsx_import_source {
            self.compiler_options.insert(
                "jsxImportSource".to_owned(),
                format!("{jsx_import_source:?}"),
            );
        }

        if let Some(module) = &compiler_options.module {
            self.compiler_options
                .insert("module".to_owned(), format!("{module:?}"));
        }

        if let Some(module_resolution) = &compiler_options.module_resolution {
            self.compiler_options.insert(
                "moduleResolution".to_owned(),
                format!("{module_resolution:?}"),
            );
        }

        if let Some(target) = &compiler_options.target {
            self.compiler_options
                .insert("target".to_owned(), format!("{target:?}"));
        }
    }
}

impl Hasher for TypeScriptTargetHasher {
    fn hash(&self, sha: &mut Sha256) {
        hash_btree(&self.compiler_options, sha);
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_hasher::to_hash;
    use moon_typescript_lang::tsconfig::{CompilerOptions, Module, ModuleResolution, Target};

    #[test]
    fn supports_all_dep_types() {
        let mut tsconfig = TsConfigJson {
            compiler_options: Some(CompilerOptions::default()),
            ..TsConfigJson::default()
        };

        tsconfig.compiler_options.as_mut().unwrap().module = Some(Module::Es2022);

        let mut hasher1 = TypeScriptTargetHasher::default();
        hasher1.hash_compiler_options(tsconfig.compiler_options.as_ref().unwrap());
        let hash1 = to_hash(&hasher1);

        tsconfig
            .compiler_options
            .as_mut()
            .unwrap()
            .module_resolution = Some(ModuleResolution::NodeNext);

        let mut hasher2 = TypeScriptTargetHasher::default();
        hasher2.hash_compiler_options(tsconfig.compiler_options.as_ref().unwrap());
        let hash2 = to_hash(&hasher2);

        tsconfig.compiler_options.as_mut().unwrap().target = Some(Target::Es2019);

        let mut hasher3 = TypeScriptTargetHasher::default();
        hasher3.hash_compiler_options(tsconfig.compiler_options.as_ref().unwrap());
        let hash3 = to_hash(&hasher3);

        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }
}
