use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use moon_common::BLOCKING_THREAD_COUNT;
use moon_hash::Digest;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

pub fn create_timestamp(time: SystemTime) -> Option<Timestamp> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        })
}

pub fn create_timestamp_from_naive(time: NaiveDateTime) -> Option<Timestamp> {
    let utc = time.and_utc();

    Some(Timestamp {
        seconds: utc.timestamp(),
        nanos: utc.timestamp_subsec_nanos() as i32,
    })
}

pub fn create_from_timestamp(timestamp: Timestamp) -> SystemTime {
    UNIX_EPOCH + Duration::new(timestamp.seconds as u64, timestamp.nanos as u32)
}

pub fn check_blob_integrity(expected_digest: &Digest, bytes: &[u8]) -> miette::Result<bool> {
    if bytes.len() != expected_digest.size as usize {
        warn!(
            hash = expected_digest.hash.as_str(),
            expected_size = expected_digest.size,
            actual_size = bytes.len(),
            "Integrity failure, mismatched file sizes, discarding blob",
        );

        return Ok(false);
    }

    let actual_digest = Digest::from_bytes(bytes)?;

    if &actual_digest != expected_digest {
        warn!(
            hash = expected_digest.hash.as_str(),
            actual_hash = actual_digest.hash.as_str(),
            "Integrity failure, mismatched digests, discarding blob",
        );

        return Ok(false);
    }

    Ok(true)
}

const BUFFER: usize = 300; // 256 (hash/digest) + 44 (metadata overhead)

pub struct Batch<T> {
    pub items: Vec<T>,
    pub size: usize,
    pub stream: bool,
    pub index: usize,
    pub total: usize,
}

pub fn partition_into_batches<T>(
    mut items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> Vec<Batch<T>> {
    // First-fit-decreasing: placing the largest items first packs batches denser
    // (fewer batches, fewer round trips) than first-fit on the incoming order.
    // The linear first-fit scan below is fine at our scale (blobs per task).
    items.sort_unstable_by_key(|item| std::cmp::Reverse(get_size(item)));

    let mut batches: Vec<Batch<T>> = vec![];

    for item in items {
        let item_size = get_size(&item) + BUFFER;

        // Item is too large to share a batch, so stream it on its own
        if item_size >= max_size {
            batches.push(Batch {
                items: vec![item],
                size: item_size,
                stream: true,
                index: 0,
                total: 0,
            });

            continue;
        }

        match batches
            .iter_mut()
            .find(|batch| !batch.stream && batch.size + item_size <= max_size)
        {
            Some(batch) => {
                batch.size += item_size;
                batch.items.push(item);
            }
            None => {
                batches.push(Batch {
                    items: vec![item],
                    size: item_size,
                    stream: false,
                    index: 0,
                    total: 0,
                });
            }
        }
    }

    let total = batches.len();

    for (index, batch) in batches.iter_mut().enumerate() {
        batch.index = index + 1;
        batch.total = total;
    }

    batches
}

pub fn chunk_into_batches<T>(mut items: Vec<T>, get_size: impl Fn(&T) -> usize) -> Vec<Batch<T>> {
    let chunk_size = items.len() / BLOCKING_THREAD_COUNT;
    let mut batches = Vec::new();

    while !items.is_empty() {
        let chunk = items
            .drain(0..chunk_size.max(1).min(items.len()))
            .collect::<Vec<_>>();

        batches.push(Batch {
            size: chunk.iter().map(&get_size).sum::<usize>(),
            items: chunk,
            stream: false,
            index: 0,
            total: 0,
        });
    }

    let total = batches.len();

    for (index, batch) in batches.iter_mut().enumerate() {
        batch.index = index + 1;
        batch.total = total;
    }

    batches
}

pub fn create_batches<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> Vec<Batch<T>> {
    if max_size > 0 {
        partition_into_batches(items, max_size, get_size)
    } else {
        chunk_into_batches(items, get_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_counts(batches: &[Batch<usize>]) -> Vec<usize> {
        batches.iter().map(|batch| batch.items.len()).collect()
    }

    mod partition_into_batches {
        use super::*;

        #[test]
        fn empty_input_yields_no_batches() {
            let batches = partition_into_batches(Vec::<usize>::new(), 1000, |n| *n);

            assert!(batches.is_empty());
        }

        #[test]
        fn packs_then_spills_by_size() {
            // Each item costs size + BUFFER (300). With max 1000, two 100-byte
            // items (800) fit; the third spills into a second batch.
            let batches = partition_into_batches(vec![100, 100, 100], 1000, |n| *n);

            assert_eq!(item_counts(&batches), vec![2, 1]);
        }

        #[test]
        fn oversized_item_becomes_its_own_stream_batch() {
            let batches = partition_into_batches(vec![5000], 1000, |n| *n);

            assert_eq!(batches.len(), 1);
            assert_eq!(batches[0].items, vec![5000]);
            assert!(batches[0].stream);
        }

        #[test]
        fn assigns_one_based_index_and_total() {
            let batches = partition_into_batches(vec![100, 100, 100], 1000, |n| *n);
            let total = batches.len();

            for (i, batch) in batches.iter().enumerate() {
                assert_eq!(batch.index, i + 1);
                assert_eq!(batch.total, total);
            }
        }
    }

    mod chunk_into_batches {
        use super::*;

        #[test]
        fn empty_input_yields_no_batches() {
            let batches = chunk_into_batches(Vec::<usize>::new(), |n| *n);

            assert!(batches.is_empty());
        }

        #[test]
        fn small_input_is_one_item_per_chunk() {
            // Fewer than BLOCKING_THREAD_COUNT items → chunk size clamps to 1,
            // maximizing parallelism for small sets.
            let batches = chunk_into_batches(vec![1, 2, 3, 4], |n| *n);

            assert_eq!(batches.len(), 4);
            assert!(batches.iter().all(|batch| batch.items.len() == 1));
        }

        #[test]
        fn large_input_is_grouped_and_preserves_every_item() {
            let items: Vec<usize> = (0..5000).collect();
            let batches = chunk_into_batches(items, |n| *n);

            let total: usize = batches.iter().map(|batch| batch.items.len()).sum();
            assert_eq!(total, 5000);
            // Grouped rather than one-per-item once the set is large.
            assert!(batches.len() > 1);
            assert!(batches.len() < 5000);
        }

        #[test]
        fn never_marks_a_batch_as_stream() {
            let batches = chunk_into_batches(vec![1, 2, 3], |n| *n);

            assert!(batches.iter().all(|batch| !batch.stream));
        }
    }

    mod create_batches {
        use super::*;

        #[test]
        fn positive_max_size_partitions_by_size() {
            // Would be one-per-chunk under chunking, but packs into 2 size
            // batches — proving size partitioning was selected.
            let batches = create_batches(vec![100, 100, 100], 1000, |n| *n);

            assert_eq!(item_counts(&batches), vec![2, 1]);
        }

        #[test]
        fn zero_max_size_chunks_across_threads() {
            let batches = create_batches(vec![100, 100, 100], 0, |n| *n);

            assert_eq!(batches.len(), 3);
            assert!(batches.iter().all(|batch| batch.items.len() == 1));
        }
    }

    mod timestamps {
        use super::*;

        #[test]
        fn round_trips_through_protobuf() {
            let now = SystemTime::now();

            let restored = create_from_timestamp(create_timestamp(now).unwrap());

            let original = now.duration_since(UNIX_EPOCH).unwrap();
            let after = restored.duration_since(UNIX_EPOCH).unwrap();
            assert_eq!(original.as_secs(), after.as_secs());
            assert_eq!(original.subsec_nanos(), after.subsec_nanos());
        }
    }
}
