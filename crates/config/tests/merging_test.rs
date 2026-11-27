use indexmap::IndexMap;
use moon_config::{MergeStrategy, merge};

mod merge_vec {
    use super::*;

    #[test]
    fn appends() {
        assert_eq!(
            merge(vec![1, 2], vec![2, 3], MergeStrategy::Append, 1, false),
            vec![1, 2, 2, 3]
        );
    }

    #[test]
    fn appends_deduped() {
        assert_eq!(
            merge(vec![1, 2], vec![2, 3, 3], MergeStrategy::Append, 1, true),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn prepends() {
        assert_eq!(
            merge(vec![1, 2], vec![2, 3], MergeStrategy::Prepend, 1, false),
            vec![2, 3, 1, 2]
        );
    }

    #[test]
    fn prepends_deduped() {
        assert_eq!(
            merge(vec![1, 2], vec![2, 3], MergeStrategy::Prepend, 1, true),
            vec![2, 3, 1]
        );
    }

    #[test]
    fn keeps_base_when_next_is_empty() {
        assert_eq!(
            merge(vec![1, 2], vec![], MergeStrategy::Append, 1, false),
            vec![1, 2]
        );
        assert_eq!(
            merge(vec![1, 2], vec![], MergeStrategy::Prepend, 1, false),
            vec![1, 2]
        );
    }

    #[test]
    fn preserves_first_link() {
        assert_eq!(
            merge(vec![], vec![1], MergeStrategy::Preserve, 0, false),
            vec![1]
        );
        assert_eq!(
            merge(vec![1], vec![2], MergeStrategy::Preserve, 1, false),
            vec![1]
        );
    }

    #[test]
    fn replaces() {
        assert_eq!(
            merge(vec![1, 2], vec![3], MergeStrategy::Replace, 1, false),
            vec![3]
        );
        assert_eq!(
            merge(vec![1, 2], vec![], MergeStrategy::Replace, 1, false),
            Vec::<i32>::new()
        );
    }
}

mod merge_index_map {
    use super::*;

    fn map(items: &[(&'static str, i32)]) -> IndexMap<&'static str, i32> {
        IndexMap::from_iter(items.iter().copied())
    }

    #[test]
    fn appends_with_next_values_winning() {
        let merged = merge(
            map(&[("a", 1), ("b", 1)]),
            map(&[("b", 2), ("c", 2)]),
            MergeStrategy::Append,
            1,
            true,
        );

        assert_eq!(
            merged.into_iter().collect::<Vec<_>>(),
            [("a", 1), ("b", 2), ("c", 2)]
        );
    }

    #[test]
    fn prepends_with_base_values_winning() {
        let merged = merge(
            map(&[("a", 1), ("b", 1)]),
            map(&[("b", 2), ("c", 2)]),
            MergeStrategy::Prepend,
            1,
            true,
        );

        assert_eq!(
            merged.into_iter().collect::<Vec<_>>(),
            [("b", 1), ("c", 2), ("a", 1)]
        );
    }

    #[test]
    fn preserves_first_link() {
        assert_eq!(
            merge(
                map(&[("a", 1)]),
                map(&[("b", 2)]),
                MergeStrategy::Preserve,
                1,
                true
            ),
            map(&[("a", 1)])
        );
    }

    #[test]
    fn replaces() {
        assert_eq!(
            merge(
                map(&[("a", 1)]),
                map(&[("b", 2)]),
                MergeStrategy::Replace,
                1,
                true
            ),
            map(&[("b", 2)])
        );
    }
}
