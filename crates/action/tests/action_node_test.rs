use moon_action::{ActionNode, RunTaskNode};
use moon_target::Target;
use rustc_hash::FxHashSet;

fn create_node(env: &[(&str, &str)]) -> ActionNode {
    let mut node = RunTaskNode::new(Target::parse("project:task").unwrap());

    for (key, value) in env {
        node.env.insert((*key).into(), Some((*value).into()));
    }

    ActionNode::run_task(node)
}

mod run_task_node {
    use super::*;

    // Maps are equal regardless of the order of their keys, so nodes that are
    // equal must hash the same, otherwise they are not found in hashed
    // collections, and the same task is inserted into the graph twice
    #[test]
    fn hashes_env_regardless_of_order() {
        let a = create_node(&[("X", "1"), ("Y", "2")]);
        let b = create_node(&[("Y", "2"), ("X", "1")]);

        assert_eq!(a, b);

        let mut set = FxHashSet::default();
        set.insert(a);

        assert!(set.contains(&b));
    }

    #[test]
    fn hashes_env_differently_for_different_values() {
        let a = create_node(&[("X", "1"), ("Y", "2")]);
        let b = create_node(&[("X", "2"), ("Y", "1")]);

        assert_ne!(a, b);

        let mut set = FxHashSet::default();
        set.insert(a);

        assert!(!set.contains(&b));
    }
}
