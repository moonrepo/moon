use moon_common::path::standardize_separators;
use moon_test_utils::{
    assert_snapshot, create_sandbox_with_config, predicates::prelude::*, Sandbox,
};
use std::fs;

fn generate_sandbox() -> Sandbox {
    create_sandbox_with_config("generator", None, None, None)
}

#[test]
fn creates_a_new_template() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("new-name").arg("--template");
    });

    let output = assert.output();

    assert!(predicate::str::contains("Created a new template new-name at").eval(&output));
    assert!(sandbox.path().join("templates/new-name").exists());

    assert.success();
}

#[test]
fn generates_files_from_template() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("test").exists());
    assert!(sandbox.path().join("test/file.ts").exists());
    assert!(sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(sandbox.path().join("test/image.jpg").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn generates_files_into_default_dest() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("dest");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("apps/foo-bar/file.txt").exists());
}

#[test]
fn doesnt_generate_files_when_dryrun() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("standard")
            .arg("./test")
            .arg("--dryRun");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(!sandbox.path().join("test").exists());
    assert!(!sandbox.path().join("test/file.ts").exists());
    assert!(!sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_forced() {
    let sandbox = generate_sandbox();

    sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("standard")
            .arg("./test")
            .arg("--force");
    });

    assert_snapshot!(assert.output_standardized());

    assert!(sandbox.path().join("test").exists());
    assert!(sandbox.path().join("test/file.ts").exists());
    assert!(sandbox.path().join("test/folder/nested-file.ts").exists());
    assert!(!sandbox.path().join("test/template.yml").exists());
}

#[test]
fn overwrites_existing_files_when_interpolated_path() {
    let sandbox = generate_sandbox();

    sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--force");
    });

    assert_snapshot!(assert.output_standardized());

    // file-[stringNotEmpty]-[number].txt
    assert!(sandbox.path().join("./test/file-default-0.txt").exists());
}

#[test]
fn renders_and_interpolates_templates() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/expressions.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/control.txt")).unwrap());
}

#[test]
fn renders_with_custom_vars_via_args() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .args([
                "--no-boolTrue",
                "--boolFalse",
                "--string=abc",
                "--stringNotEmpty",
                "xyz",
                "--number=123",
                "--numberNotEmpty",
                "456",
                "--enum=c",
                "--multenumNotEmpty",
                "a",
            ]);
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/expressions.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/control.txt")).unwrap());
}

#[test]
fn cant_overwrite_internal_variables_with_args() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .args(["--internal", "overwritten"]);
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/expressions.txt")).unwrap());
}

#[test]
fn handles_raw_files() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate").arg("standard").arg("./test");
    });

    assert.success();

    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/file.txt")).unwrap());
    assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/other.txt")).unwrap());
}

#[test]
fn interpolates_destination_path() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    // Verify output paths are correct
    assert_snapshot!(assert.output_standardized());

    // file-[stringNotEmpty]-[number].txt
    assert!(sandbox.path().join("./test/file-default-0.txt").exists());
}

#[test]
fn errors_when_parsing_custom_var_types() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults")
            .arg("--")
            .arg("--number=abc");
    });

    assert_snapshot!(assert.output_standardized());
}

#[test]
fn supports_custom_filters() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("vars")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    let content = fs::read_to_string(sandbox.path().join("./test/filters.txt")).unwrap();

    assert_snapshot!(standardize_separators(content));
}

#[test]
fn supports_tera_twig_exts() {
    let sandbox = generate_sandbox();

    let assert = sandbox.run_moon(|cmd| {
        cmd.arg("generate")
            .arg("extensions")
            .arg("./test")
            .arg("--defaults");
    });

    assert.success();

    let tera = sandbox.path().join("./test/file.ts");
    let twig = sandbox.path().join("./test/file.tsx");

    assert!(tera.exists());
    assert!(twig.exists());

    assert_eq!(
        fs::read_to_string(tera).unwrap(),
        "export type FooBar = true;\n"
    );
    assert_eq!(
        fs::read_to_string(twig).unwrap(),
        "export type FooBar = true;\n"
    );
}

mod extends {
    use super::*;

    #[test]
    fn generates_files_from_all_templates() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate").arg("extends").arg("./test");
        });

        assert_snapshot!(assert.output_standardized());

        assert!(sandbox.path().join("test").exists());
        assert!(sandbox.path().join("test/base.txt").exists());
        assert!(sandbox.path().join("test/one.txt").exists());
        assert!(sandbox.path().join("test/two.txt").exists());
        assert!(sandbox.path().join("test/vars.txt").exists());
    }

    #[test]
    fn primary_files_overwrite_extended_files() {
        let sandbox = generate_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("generate").arg("extends").arg("./test");
        });

        assert_eq!(
            fs::read_to_string(sandbox.path().join("test/two.txt")).unwrap(),
            "two overwritten\n"
        );
    }

    #[test]
    fn primary_file_can_use_vars_from_extended() {
        let sandbox = generate_sandbox();

        sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("extends")
                .arg("./test")
                .arg("--")
                .arg("--one")
                .arg("abc")
                .arg("--two")
                .arg("123");
        });

        assert_snapshot!(fs::read_to_string(sandbox.path().join("test/vars.txt")).unwrap());
    }
}

mod frontmatter {
    use super::*;

    #[test]
    fn changes_dest_path() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(!sandbox.path().join("./test/to.txt").exists());
        assert!(sandbox.path().join("./test/to-NEW.txt").exists());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/to-NEW.txt")).unwrap());
    }

    #[test]
    fn force_writes_file() {
        let sandbox = generate_sandbox();

        sandbox.create_file("test/forced.txt", "Original content");

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert_snapshot!(fs::read_to_string(sandbox.path().join("./test/forced.txt")).unwrap());
    }

    #[test]
    fn skips_over_file() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(!sandbox.path().join("./test/skipped.txt").exists());
    }

    #[test]
    fn supports_component_vars() {
        let sandbox = generate_sandbox();

        let assert = sandbox.run_moon(|cmd| {
            cmd.arg("generate")
                .arg("frontmatter")
                .arg("./test")
                .arg("--defaults");
        });

        assert.success();

        assert!(sandbox
            .path()
            .join("./test/components/SmallButton.tsx")
            .exists());
        assert_snapshot!(fs::read_to_string(
            sandbox.path().join("./test/components/SmallButton.tsx")
        )
        .unwrap());
    }
}
