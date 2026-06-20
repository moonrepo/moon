use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

const BUFFER: usize = 300; // 256 (hash/digest) + 44 (metadata overhead)

pub struct Partition<T> {
    pub items: Vec<T>,
    pub size: usize,
    pub stream: bool,
}

pub fn partition_into_batches<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> BTreeMap<i32, Partition<T>> {
    let mut batches = BTreeMap::<i32, Partition<T>>::default();

    for item in items {
        let item_size = get_size(&item) + BUFFER;
        let mut index_to_use = -1;
        let mut stream = false;

        // Item is too large, must be streamed
        if item_size >= max_size {
            stream = true;
        }
        // Try and find a partition that this item can go into
        else {
            for (index, batch) in &batches {
                if !batch.stream && (batch.size + item_size) <= max_size {
                    index_to_use = *index;
                    break;
                }
            }
        }

        // If no partition available, create a new one
        if index_to_use == -1 {
            index_to_use = batches.len() as i32;
        }

        let batch = batches.entry(index_to_use).or_insert_with(|| Partition {
            items: vec![],
            size: 0,
            stream: false,
        });
        batch.size += item_size;
        batch.stream = stream;
        batch.items.push(item);
    }

    batches
}
