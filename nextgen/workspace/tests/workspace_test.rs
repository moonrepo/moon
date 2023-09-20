use moon_workspace::Workspace;
use starbase_sandbox::create_empty_sandbox;

#[test]
fn loads_proto_tools() {
    let sandbox = create_empty_sandbox();

    sandbox.create_file(".moon/workspace.yml", "");

    sandbox.create_file(
        ".prototools",
        r#"
node = "18.0.0"
npm = "9.0.0"
"#,
    );

    let workspace = Workspace::load_from(sandbox.path()).unwrap();

    assert_eq!(
        workspace.proto_tools.tools.get("node").unwrap().to_string(),
        "18.0.0"
    );
    assert_eq!(
        workspace.proto_tools.tools.get("npm").unwrap().to_string(),
        "9.0.0"
    );
}
