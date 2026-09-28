#[cfg(feature = "api-docs")]
use moon_config_schema::api_docs::generate_api_docs;
use moon_config_schema::json_schemas::generate_json_schemas;
use moon_config_schema::pkl_schemas::generate_pkl_schemas;
#[cfg(feature = "typescript")]
use moon_config_schema::typescript_types::generate_typescript_types;
use std::env;
use std::process::Command;

fn main() {
    let cwd = env::current_dir().unwrap();

    generate_json_schemas(cwd.join("website/static/schemas/v2"), Default::default()).unwrap();

    generate_pkl_schemas(cwd.join("website/static/pkl/v2")).unwrap();

    #[cfg(feature = "api-docs")]
    {
        for lookup in [
            "../moonrepo/website/web/content/api/moon",
            "../website/web/content/api/moon",
            "temp/moon-api-docs",
        ] {
            let dir = cwd.join(lookup);

            if dir.exists() {
                generate_api_docs(dir).unwrap();
                break;
            }
        }
    }

    #[cfg(feature = "typescript")]
    generate_typescript_types(cwd.join("packages/types/src")).unwrap();

    // Run formatter
    let mut cmd = Command::new("vp");
    cmd.args(["fmt"]);
    cmd.current_dir(cwd);
    let _ = cmd.output();
}
