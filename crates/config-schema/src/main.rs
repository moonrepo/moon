use moon_config_schema::json_schemas::generate_json_schemas;
#[cfg(feature = "typescript")]
use moon_config_schema::typescript_types::generate_typescript_types;
use std::env;
use std::process::Command;

fn main() {
    let cwd = env::current_dir().unwrap();

    generate_json_schemas(cwd.join("website/static/schemas"), Default::default()).unwrap();

    #[cfg(feature = "typescript")]
    generate_typescript_types(cwd.join("packages/types/src")).unwrap();

    // Run prettier
    let prettier = cwd.join("node_modules/.bin/prettier");

    if prettier.exists() {
        let mut cmd = Command::new(prettier);
        cmd.args(["--write", "packages/types"]);
        cmd.current_dir(cwd);
        cmd.output().unwrap();
    }
}
