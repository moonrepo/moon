use assert_fs::TempDir;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fake::{Dummy, Fake, Faker};
use moon_archive::{tar, untar, untar_with_diff, TreeDiffer};
use moon_error::MoonError;
use moon_utils::string_vec;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_tree() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let mut dir = 'a';

    for i in 1..1000 {
        fs::write(
            temp_dir
                .path()
                .join("sources")
                .join(String::from(dir))
                .join(format!("{}.txt", i)),
            Faker.fake::<String>(),
        )
        .unwrap();

        if i % 100 == 0 {
            dir = match i {
                100 => 'b',
                200 => 'c',
                300 => 'd',
                400 => 'e',
                500 => 'f',
                600 => 'g',
                700 => 'h',
                800 => 'i',
                900 => 'j',
                1000 => 'k',
                _ => 'a',
            };
        }
    }

    temp_dir
}

pub fn tar_benchmark(c: &mut Criterion) {
    let temp_dir = create_tree();
    let dirs = string_vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k'];
    let sources = temp_dir.path().join("sources");
    let archive_file = temp_dir.path().join("archive.tar.gz");

    tar(&sources, &dirs, &archive_file, None).unwrap();

    c.bench_function("tar", |b| {
        b.iter(|| {
            untar(&archive_file, &sources, None).unwrap();
        })
    });

    c.bench_function("tar_diff", |b| {
        b.iter(|| {
            let mut diff = TreeDiffer::load(&sources, &dirs).unwrap();

            untar_with_diff(&mut diff, &archive_file, &sources, None).unwrap();
        })
    });

    // let runtime = tokio::runtime::Runtime::new().unwrap();
}

// pub fn tar_benchmark(c: &mut Criterion) {
//     let temp_dir = create_tree();
//     let runtime = tokio::runtime::Runtime::new().unwrap();

//     c.bench_function("emitter_emit", |b| {
//         b.to_async(&runtime).iter(|| async {
//             black_box(emitter);
//         })
//     });
// }

criterion_group!(tar_archive, tar_benchmark);
criterion_main!(tar_archive);
