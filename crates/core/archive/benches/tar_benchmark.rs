// Allow `temp_dir` so that files arent removed when dropping scope
#![allow(unused_variables)]

use criterion::{criterion_group, criterion_main, Criterion};
use fake::{Fake, Faker};
use moon_archive::{tar, untar, untar_with_diff, TreeDiffer};
use moon_test_utils::{assert_fs::TempDir, create_temp_dir};
use moon_utils::string_vec;
use std::fs;
use std::path::PathBuf;

fn create_tree() -> (TempDir, PathBuf, PathBuf, Vec<String>) {
    let temp_dir = create_temp_dir();
    let sources_dir = temp_dir.path().join("sources");
    let archive_file = temp_dir.path().join("archive.tar.gz");
    let dirs = string_vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k'];
    let mut dir_index = 0;

    for i in 1..1000 {
        let parent = sources_dir.join(&dirs[dir_index]);

        if !parent.exists() {
            fs::create_dir_all(&parent).unwrap();
        }

        fs::write(parent.join(format!("{i}.txt")), Faker.fake::<String>()).unwrap();

        if i % 100 == 0 {
            dir_index += 1;
        }
    }

    (temp_dir, sources_dir, archive_file, dirs)
}

pub fn tar_base_benchmark(c: &mut Criterion) {
    let (temp_dir, sources_dir, archive_file, dirs) = create_tree();

    tar(&sources_dir, &dirs, &archive_file, None).unwrap();

    c.bench_function("tar_base", |b| {
        b.iter(|| {
            untar(&archive_file, &sources_dir, None).unwrap();
        })
    });
}

pub fn tar_base_remove_benchmark(c: &mut Criterion) {
    let (temp_dir, sources_dir, archive_file, dirs) = create_tree();

    tar(&sources_dir, &dirs, &archive_file, None).unwrap();

    c.bench_function("tar_base_remove", |b| {
        b.iter(|| {
            fs::remove_dir_all(&sources_dir).unwrap();

            untar(&archive_file, &sources_dir, None).unwrap();
        })
    });
}

pub fn tar_diff_benchmark(c: &mut Criterion) {
    let (temp_dir, sources_dir, archive_file, dirs) = create_tree();

    tar(&sources_dir, &dirs, &archive_file, None).unwrap();

    c.bench_function("tar_diff", |b| {
        b.iter(|| {
            let mut diff = TreeDiffer::load(&sources_dir, &dirs).unwrap();

            untar_with_diff(&mut diff, &archive_file, &sources_dir, None).unwrap();
        })
    });
}

pub fn tar_diff_remove_benchmark(c: &mut Criterion) {
    let (temp_dir, sources_dir, archive_file, dirs) = create_tree();

    tar(&sources_dir, &dirs, &archive_file, None).unwrap();

    c.bench_function("tar_diff_remove", |b| {
        b.iter(|| {
            fs::remove_dir_all(&sources_dir).unwrap();

            let mut diff = TreeDiffer::load(&sources_dir, &dirs).unwrap();

            untar_with_diff(&mut diff, &archive_file, &sources_dir, None).unwrap();
        })
    });
}

criterion_group!(
    tar_archive,
    tar_base_benchmark,
    tar_base_remove_benchmark,
    tar_diff_benchmark,
    tar_diff_remove_benchmark,
);
criterion_main!(tar_archive);
