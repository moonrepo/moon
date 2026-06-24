use bazel_remote_apis::google::protobuf::Timestamp;
// use chrono::NaiveDateTime;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn create_timestamp(time: SystemTime) -> Option<Timestamp> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        })
}

// pub fn create_timestamp_from_naive(time: NaiveDateTime) -> Option<Timestamp> {
//     let utc = time.and_utc();

//     Some(Timestamp {
//         seconds: utc.timestamp(),
//         nanos: utc.timestamp_subsec_nanos() as i32,
//     })
// }

pub fn create_from_timestamp(timestamp: Timestamp) -> SystemTime {
    UNIX_EPOCH + Duration::new(timestamp.seconds as u64, timestamp.nanos as u32)
}

const BUFFER: usize = 300; // 256 (hash/digest) + 44 (metadata overhead)

pub struct Partition<T> {
    pub items: Vec<T>,
    pub size: usize,
    pub stream: bool,
    pub index: usize,
    pub total: usize,
}

pub fn partition_into_batches<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> Vec<Partition<T>> {
    let mut batches: Vec<Partition<T>> = vec![];

    for item in items {
        let item_size = get_size(&item) + BUFFER;

        // Item is too large to share a batch, so stream it on its own.
        if item_size >= max_size {
            batches.push(Partition {
                items: vec![item],
                size: item_size,
                stream: true,
                index: 0,
                total: 0,
            });

            continue;
        }

        // First-fit: drop it into the first non-stream batch that has room,
        // otherwise start a new batch.
        match batches
            .iter_mut()
            .find(|batch| !batch.stream && batch.size + item_size <= max_size)
        {
            Some(batch) => {
                batch.size += item_size;
                batch.items.push(item);
            }
            None => {
                batches.push(Partition {
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
