use insta::assert_snapshot;
use moon_utils::test::{create_moon_command, get_assert_output};

mod node {
    use super::*;

    #[test]
    fn runs_standard_script() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:standard")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:processExitNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_zero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }

    #[test]
    fn handles_process_exit_code_nonzero() {
        let assert = create_moon_command("cases")
            .arg("run")
            .arg("node:exitCodeNonZero")
            .assert();

        assert_snapshot!(get_assert_output(&assert));
    }
}
