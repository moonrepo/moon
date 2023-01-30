use moon_archive::TreeDiffer;
use moon_test_utils::create_sandbox;
use moon_utils::string_vec;
use std::fs::{self, File};

#[test]
fn loads_all_files() {
    let sandbox = create_sandbox("generator");
    let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates"]).unwrap();

    assert_eq!(differ.files.len(), 21);
}

#[test]
fn loads_using_globs() {
    let sandbox = create_sandbox("generator");
    let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates/**/*"]).unwrap();

    assert_eq!(differ.files.len(), 21);
}

#[test]
fn removes_stale_files() {
    let sandbox = create_sandbox("generator");
    let mut differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates"]).unwrap();

    // Delete everything, hah
    differ.remove_stale_tracked_files();

    let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates"]).unwrap();

    assert_eq!(differ.files.len(), 0);
}

mod equal_check {
    use super::*;

    #[test]
    fn returns_true_if_equal() {
        let sandbox = create_sandbox("generator");
        let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates"]).unwrap();

        let source_path = sandbox.path().join("templates/standard/file-source.txt");
        fs::write(&source_path, "content").unwrap();
        let mut source = File::open(&source_path).unwrap();

        let dest_path = sandbox.path().join("templates/standard/file.txt");
        fs::write(&dest_path, "content").unwrap();
        let mut dest = File::open(&dest_path).unwrap();

        assert!(differ.are_files_equal(&mut source, &mut dest).unwrap());
    }

    #[test]
    fn returns_false_if_diff_sizes() {
        let sandbox = create_sandbox("generator");
        let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates/**/*"]).unwrap();

        let source_path = sandbox.path().join("templates/standard/file-source.txt");
        fs::write(&source_path, "data").unwrap();
        let mut source = File::open(&source_path).unwrap();

        let dest_path = sandbox.path().join("templates/standard/file.txt");
        fs::write(&dest_path, "content").unwrap();
        let mut dest = File::open(&dest_path).unwrap();

        assert!(!differ.are_files_equal(&mut source, &mut dest).unwrap());
    }

    #[test]
    fn returns_false_if_diff_data() {
        let sandbox = create_sandbox("generator");
        let differ = TreeDiffer::load(sandbox.path(), &string_vec!["templates"]).unwrap();

        let source_path = sandbox.path().join("templates/standard/file-source.txt");
        fs::write(&source_path, "cont...").unwrap();
        let mut source = File::open(&source_path).unwrap();

        let dest_path = sandbox.path().join("templates/standard/file.txt");
        fs::write(&dest_path, "content").unwrap();
        let mut dest = File::open(&dest_path).unwrap();

        assert!(!differ.are_files_equal(&mut source, &mut dest).unwrap());
    }
}
