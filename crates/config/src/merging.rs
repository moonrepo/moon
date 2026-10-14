use crate::config_unit_enum;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{ConfigEnum, MergeError, MergeResult, PartialConfig};
use std::collections::BTreeMap;
use std::hash::Hash;

config_unit_enum!(
    /// The strategy in which to merge two configuration values.
    #[derive(ConfigEnum, Default)]
    pub enum MergeStrategy {
        #[default]
        Append,
        Prepend,
        Preserve,
        Replace,
    }
);

/// A collection that can be merged with [`merge`].
pub trait Mergeable: IntoIterator {
    fn is_empty(&self) -> bool;

    fn merge_item(&mut self, item: Self::Item, dedupe: bool);
}

impl<T: PartialEq> Mergeable for Vec<T> {
    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }

    // Dedupe while merging vectors. We can't use a set here because
    // we need to preserve the insertion order. Revisit if this is costly!
    fn merge_item(&mut self, item: T, dedupe: bool) {
        if !dedupe || !self.contains(&item) {
            self.push(item);
        }
    }
}

impl<K: Eq + Hash, V> Mergeable for IndexMap<K, V> {
    fn is_empty(&self) -> bool {
        IndexMap::is_empty(self)
    }

    // Maps are always deduped by key, with the latest value winning.
    fn merge_item(&mut self, (key, value): (K, V), _dedupe: bool) {
        self.insert(key, value);
    }
}

/// Merge two collections together based on the provided strategy. The index
/// is the position of `next` within the inheritance chain, where 0 is
/// the first (most global) link, which the preserve strategy keeps.
pub fn merge<T: Mergeable>(
    base: T,
    next: T,
    strategy: MergeStrategy,
    index: usize,
    dedupe: bool,
) -> T {
    let (mut merged, rest) = match strategy {
        MergeStrategy::Append | MergeStrategy::Prepend if next.is_empty() => return base,
        MergeStrategy::Append => (base, next),
        MergeStrategy::Prepend => (next, base),
        MergeStrategy::Preserve => return if index == 0 { next } else { base },
        MergeStrategy::Replace => return next,
    };

    for item in rest {
        merged.merge_item(item, dedupe);
    }

    merged
}

/// Schematic merge function that extends the previous collection with the next.
pub fn merge_iter<I, V, C>(mut prev: I, next: I, _: &C) -> MergeResult<I>
where
    I: Extend<V> + IntoIterator<Item = V>,
{
    prev.extend(next);
    Ok(Some(prev))
}

/// A map of partial configs that can be merged with [`merge_partials`].
pub trait PartialsMap<K, V>: IntoIterator<Item = (K, V)> {
    fn get_partial_mut(&mut self, key: &K) -> Option<&mut V>;

    fn insert_partial(&mut self, key: K, value: V);
}

impl<K: Eq + Hash, V> PartialsMap<K, V> for FxHashMap<K, V> {
    fn get_partial_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_mut(key)
    }

    fn insert_partial(&mut self, key: K, value: V) {
        self.insert(key, value);
    }
}

impl<K: Ord, V> PartialsMap<K, V> for BTreeMap<K, V> {
    fn get_partial_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_mut(key)
    }

    fn insert_partial(&mut self, key: K, value: V) {
        self.insert(key, value);
    }
}

/// Schematic merge function that deeply merges each partial config in the
/// next map into the partial config with the same key in the previous map.
pub fn merge_partials<M, K, V>(mut prev: M, next: M, context: &V::Context) -> MergeResult<M>
where
    M: PartialsMap<K, V>,
    V: PartialConfig,
{
    for (key, value) in next {
        match prev.get_partial_mut(&key) {
            Some(existing) => {
                existing.merge(context, value).map_err(MergeError::new)?;
            }
            None => {
                prev.insert_partial(key, value);
            }
        }
    }

    Ok(Some(prev))
}
