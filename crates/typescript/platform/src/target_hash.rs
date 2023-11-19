use moon_config::TypeScriptConfig;
use moon_hash::hash_content;
use moon_typescript_lang::tsconfig::{CompilerOptions, TsConfigJson};
use std::{collections::BTreeMap, path::Path};

hash_content!(
    #[derive(Default)]
    pub struct TypeScriptTargetHash {
        // `tsconfig.json` `compilerOptions`
        compiler_options: BTreeMap<String, String>,
    }
);

impl TypeScriptTargetHash {
    pub fn generate(
        config: &TypeScriptConfig,
        workspace_root: &Path,
        project_root: &Path,
    ) -> miette::Result<TypeScriptTargetHash> {
        let mut hasher = TypeScriptTargetHash::default();

        if let Some(root_tsconfig) = TsConfigJson::read_with_name(
            workspace_root.join(&config.types_root),
            &config.root_config_file_name,
        )? {
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
