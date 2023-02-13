use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_pipeline::estimator::Estimator;
use moon_platform::Runtime;
use rustc_hash::FxHashMap;
use std::time::Duration;

#[test]
fn returns_loss_state() {
    let est = Estimator::calculate(&[], Duration::new(5, 0));

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(0, 0),
            loss: Some(Duration::new(5, 0)),
            tasks: FxHashMap::default(),
            gain: None,
            percent: -100.0,
        },
    )
}

#[test]
fn returns_gain_state() {
    let est = Estimator::calculate(
        &[Action {
            duration: Some(Duration::new(10, 0)),
            node: Some(ActionNode::RunTarget(Runtime::System, "proj:task".into())),
            ..Action::default()
        }],
        Duration::new(5, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(10, 0),
            loss: None,
            tasks: FxHashMap::from_iter([("task".into(), Duration::new(10, 0))]),
            gain: Some(Duration::new(5, 0)),
            percent: 50.0,
        },
    )
}

#[test]
fn buckets_and_aggregates_tasks() {
    let est = Estimator::calculate(
        &[
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(5, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:lint".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(15, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "b:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(8, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "c:test".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(12, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "d:lint".into())),
                ..Action::default()
            },
        ],
        Duration::new(10, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(25, 0),
            loss: None,
            tasks: FxHashMap::from_iter([
                ("build".into(), Duration::new(25, 0)),
                ("lint".into(), Duration::new(17, 0)),
                ("test".into(), Duration::new(8, 0))
            ]),
            gain: Some(Duration::new(15, 0)),
            percent: 60.000004,
        },
    )
}

#[test]
fn includes_setup_install() {
    let est = Estimator::calculate(
        &[
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::SetupTool(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(25, 0)),
                node: Some(ActionNode::InstallDeps(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "proj:task".into())),
                ..Action::default()
            },
        ],
        Duration::new(5, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(45, 0),
            loss: None,
            tasks: FxHashMap::from_iter([
                ("*".into(), Duration::new(35, 0)),
                ("task".into(), Duration::new(10, 0))
            ]),
            gain: Some(Duration::new(40, 0)),
            percent: 88.88889,
        },
    )
}

#[test]
fn multiplies_cached() {
    let est = Estimator::calculate(
        &[Action {
            duration: Some(Duration::new(3, 0)),
            node: Some(ActionNode::RunTarget(Runtime::System, "proj:task".into())),
            status: ActionStatus::Cached,
            ..Action::default()
        }],
        Duration::new(5, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(30, 0),
            loss: None,
            tasks: FxHashMap::from_iter([("task".into(), Duration::new(30, 0))]),
            gain: Some(Duration::new(25, 0)),
            percent: 83.33333,
        },
    )
}

#[test]
fn calculates_gain() {
    let est = Estimator::calculate(
        &[
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::SetupTool(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(25, 0)),
                node: Some(ActionNode::InstallDeps(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(5, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:lint".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(15, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "b:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(8, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "c:test".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(12, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "d:lint".into())),
                ..Action::default()
            },
        ],
        Duration::new(25, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(60, 0),
            loss: None,
            tasks: FxHashMap::from_iter([
                ("*".into(), Duration::new(35, 0)),
                ("build".into(), Duration::new(25, 0)),
                ("lint".into(), Duration::new(17, 0)),
                ("test".into(), Duration::new(8, 0))
            ]),
            gain: Some(Duration::new(35, 0)),
            percent: 58.333332,
        },
    )
}

#[test]
fn calculates_loss() {
    let est = Estimator::calculate(
        &[
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::SetupTool(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(25, 0)),
                node: Some(ActionNode::InstallDeps(Runtime::System)),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(10, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(5, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "a:lint".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(15, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "b:build".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(8, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "c:test".into())),
                ..Action::default()
            },
            Action {
                duration: Some(Duration::new(12, 0)),
                node: Some(ActionNode::RunTarget(Runtime::System, "d:lint".into())),
                ..Action::default()
            },
        ],
        Duration::new(85, 0),
    );

    assert_eq!(
        est,
        Estimator {
            duration: Duration::new(60, 0),
            loss: Some(Duration::new(25, 0)),
            tasks: FxHashMap::from_iter([
                ("*".into(), Duration::new(35, 0)),
                ("build".into(), Duration::new(25, 0)),
                ("lint".into(), Duration::new(17, 0)),
                ("test".into(), Duration::new(8, 0))
            ]),
            gain: None,
            percent: -29.411766,
        },
    )
}
