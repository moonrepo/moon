use moon_test_utils::{assert_fs::prelude::*, create_sandbox_with_config};
use moon_workspace::Workspace;

#[test]
fn loads_proto_tools() {
    let temp = create_sandbox_with_config("base", None, None, None);

    temp.fixture
        .child(".prototools")
        .write_str(
            r#"
node = "18.0.0"
npm = "9.0.0"
"#,
        )
        .unwrap();

    let workspace = Workspace::load_from(temp.path()).unwrap();

    assert_eq!(workspace.proto_tools.tools.get("node").unwrap(), "18.0.0");
    assert_eq!(workspace.proto_tools.tools.get("npm").unwrap(), "9.0.0");
}
