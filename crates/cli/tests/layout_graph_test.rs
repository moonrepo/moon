use moon_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use starbase_sandbox::assert_snapshot;
use starbase_utils::json::serde_json;

mod layout_graph {
    use super::*;

    #[test]
    fn no_projects() {
        let sandbox = create_empty_moon_sandbox();

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("--info");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn renders_every_root() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("--info");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn renders_a_single_project() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("b").arg("--info");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_dot() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("--dot");
        });

        assert_snapshot!(assert.output());
    }

    #[test]
    fn prints_json() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("--json");
        });

        let json: serde_json::Value = serde_json::from_str(assert.stdout().trim()).unwrap();
        let roots = json["roots"].as_array().unwrap();

        assert!(json["complete"].as_bool().unwrap());
        assert_eq!(
            roots
                .iter()
                .filter(|root| root["toolchain"] == "tc-tier2")
                .map(|root| root["root"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["", "nested", "standalone"]
        );
        assert_eq!(
            json["bindings"]["tools"][0]["placement"],
            serde_json::json!({ "type": "outside", "root": "" })
        );
        assert_eq!(
            json["bindings"]["a"][0]["placement"]["role"],
            "workspaceMember"
        );
    }

    #[test]
    fn errors_for_an_unknown_project() {
        let sandbox = create_moon_sandbox("layout");

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("layout-graph").arg("unknown").arg("--info");
        });

        assert.failure();
    }
}
