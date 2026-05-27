use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub struct Partition<T> {
    pub items: Vec<T>,
    pub size: usize,
    pub stream: bool,
}

pub fn partition_into_groups<T>(
    items: Vec<T>,
    max_size: usize,
    get_size: impl Fn(&T) -> usize,
) -> BTreeMap<i32, Partition<T>> {
    let mut groups = BTreeMap::<i32, Partition<T>>::default();

    // Subtract a chunk from the max size, because when down/uploading blobs,
    // we need to account for the non-blob data in the request/response, like the
    // compression level, digest strings, status fields, etc. All of these "add up"
    // and can bump the total body size larger than the actual limit. To be safe,
    // we reduce the max size by 25%.
    let max_group_size = (max_size as f64 * 0.75) as usize;

    for item in items {
        let item_size = get_size(&item);
        let mut index_to_use = -1;
        let mut stream = false;

        // Item is too large, must be streamed
        if item_size >= max_group_size {
            stream = true;
        }
        // Try and find a partition that this item can go into
        else {
            for (index, group) in &groups {
                if !group.stream && (group.size + item_size) <= max_group_size {
                    index_to_use = *index;
                    break;
                }
            }
        }

        // If no partition available, create a new one
        if index_to_use == -1 {
            index_to_use = groups.len() as i32;
        }

        let group = groups.entry(index_to_use).or_insert_with(|| Partition {
            items: vec![],
            size: 0,
            stream: false,
        });
        group.size += item_size;
        group.stream = stream;
        group.items.push(item);
    }

    groups
}
