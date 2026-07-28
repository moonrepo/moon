use moon_action::Operation;

mod finish_from_output {
    use super::*;

    #[test]
    fn preserves_utf8_output() {
        let mut operation = Operation::task_execution("pytest");

        operation.finish_from_output(
            None,
            b"tests passed\n".to_vec(),
            b"warning issued\n".to_vec(),
        );

        let output = operation.get_exec_output().unwrap();

        assert_eq!(output.stdout.as_deref().unwrap(), "tests passed\n");
        assert_eq!(output.stderr.as_deref().unwrap(), "warning issued\n");
    }

    #[test]
    fn preserves_non_utf8_output_lossily() {
        let mut operation = Operation::task_execution("pytest");

        // 0xFC is `ü` in Windows codepage 1252, but invalid UTF-8
        operation.finish_from_output(
            None,
            b"person=\"Hans M\xfcller\",\nFAILED tests\n".to_vec(),
            b"locator(\"Absenden f\xfcr alle\")\n".to_vec(),
        );

        let output = operation.get_exec_output().unwrap();

        assert_eq!(
            output.stdout.as_deref().unwrap(),
            "person=\"Hans M\u{fffd}ller\",\nFAILED tests\n"
        );
        assert_eq!(
            output.stderr.as_deref().unwrap(),
            "locator(\"Absenden f\u{fffd}r alle\")\n"
        );
    }
}
