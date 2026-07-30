use crate::config_unit_enum;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::ConfigEnum;
use std::hash::Hash;

config_unit_enum!(
    /// The strategy in which to merge two configuration values.
    #[derive(ConfigEnum)]
    pub enum MergeStrategy {
        #[default]
        Append,
        Prepend,
        Preserve,
        Replace,
    }
);

/// Merge two maps together based on the provided strategy. The index
/// is the position of `next` within the inheritance chain, where 0 is
/// the first (most global) link, which the preserve strategy keeps.
pub fn merge_map<K, V>(
    base: FxHashMap<K, V>,
    next: FxHashMap<K, V>,
    strategy: MergeStrategy,
    index: usize,
) -> FxHashMap<K, V>
where
    K: Eq + Hash,
{
    match strategy {
        MergeStrategy::Append => {
            if next.is_empty() {
                return base;
            }

            let mut map = FxHashMap::default();
            map.extend(base);
            map.extend(next);
            map
        }
        MergeStrategy::Prepend => {
            if next.is_empty() {
                return base;
            }

            let mut map = FxHashMap::default();
            map.extend(next);
            map.extend(base);
            map
        }
        MergeStrategy::Preserve => {
            if index == 0 {
                next
            } else {
                base
            }
        }
        MergeStrategy::Replace => next,
    }
}

/// Merge two ordered maps together based on the provided strategy. The index
/// is the position of `next` within the inheritance chain, where 0 is
/// the first (most global) link, which the preserve strategy keeps.
pub fn merge_index_map<K, V>(
    base: IndexMap<K, V>,
    next: IndexMap<K, V>,
    strategy: MergeStrategy,
    index: usize,
) -> IndexMap<K, V>
where
    K: Eq + Hash,
{
    match strategy {
        MergeStrategy::Append => {
            if next.is_empty() {
                return base;
            }

            let mut map = IndexMap::default();
            map.extend(base);
            map.extend(next);
            map
        }
        MergeStrategy::Prepend => {
            if next.is_empty() {
                return base;
            }

            let mut map = IndexMap::default();
            map.extend(next);
            map.extend(base);
            map
        }
        MergeStrategy::Preserve => {
            if index == 0 {
                next
            } else {
                base
            }
        }
        MergeStrategy::Replace => next,
    }
}

/// Merge two lists together based on the provided strategy. The index
/// is the position of `next` within the inheritance chain, where 0 is
/// the first (most global) link, which the preserve strategy keeps.
pub fn merge_vec<T: Eq + std::fmt::Debug>(
    base: Vec<T>,
    next: Vec<T>,
    strategy: MergeStrategy,
    index: usize,
    dedupe: bool,
) -> Vec<T> {
    let mut list: Vec<T> = vec![];

    // Dedupe while merging vectors. We can't use a set here because
    // we need to preserve the insertion order. Revisit if this is costly!
    let mut append = |items: Vec<T>, force: bool| {
        for item in items {
            #[allow(clippy::nonminimal_bool)]
            if force || !dedupe || (dedupe && !list.contains(&item)) {
                list.push(item);
            }
        }
    };

    match strategy {
        MergeStrategy::Append => {
            if next.is_empty() {
                return base;
            }

            append(base, true);
            append(next, false);
        }
        MergeStrategy::Prepend => {
            if next.is_empty() {
                return base;
            }

            append(next, true);
            append(base, false);
        }
        MergeStrategy::Preserve => {
            if index == 0 {
                list.extend(next);
            } else {
                list.extend(base);
            }
        }
        MergeStrategy::Replace => {
            list.extend(next);
        }
    }

    list
}
