use moon_config_schema::json_schemas::generate_json_schemas;
#[cfg(feature = "typescript")]
use moon_config_schema::typescript_types::generate_typescript_types;
use std::env;
use std::process::Command;

fn main() {
    let cwd = env::current_dir().unwrap();

    generate_json_schemas(cwd.join("website/static/schemas/v2"), Default::default()).unwrap();

    #[cfg(feature = "typescript")]
    generate_typescript_types(cwd.join("packages/types/src")).unwrap();

    // Run formatter
    let mut cmd = Command::new("vp");
    cmd.args(["fmt"]);
    cmd.current_dir(cwd);
    let _ = cmd.output();
}
