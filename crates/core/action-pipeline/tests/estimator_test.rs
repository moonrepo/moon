use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_pipeline::estimator::{Estimator, TaskEstimate};
use moon_platform::Runtime;
use rustc_hash::FxHashMap;
use std::time::Duration;

const NANOS_PER_MILLI: u32 = 1_000_000;
const HALF_SECOND: u32 = NANOS_PER_MILLI * 500;

mod estimator {
    use super::*;

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
                duration: Duration::new(8, HALF_SECOND),
                loss: None,
                tasks: FxHashMap::from_iter([(
                    "task".into(),
                    TaskEstimate::new(Duration::new(10, 0))
                )]),
                gain: Some(Duration::new(3, HALF_SECOND)),
                percent: 41.17647,
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
                duration: Duration::new(42, HALF_SECOND),
                loss: None,
                tasks: FxHashMap::from_iter([
                    (
                        "build".into(),
                        TaskEstimate::with_count(Duration::new(25, 0), 2)
                    ),
                    (
                        "lint".into(),
                        TaskEstimate::with_count(Duration::new(17, 0), 2)
                    ),
                    ("test".into(), TaskEstimate::new(Duration::new(8, 0)))
                ]),
                gain: Some(Duration::new(32, HALF_SECOND)),
                percent: 76.47059,
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
                duration: Duration::new(43, HALF_SECOND),
                loss: None,
                tasks: FxHashMap::from_iter([
                    (
                        "*".into(),
                        TaskEstimate::with_count(Duration::new(35, 0), 0)
                    ),
                    ("task".into(), TaskEstimate::new(Duration::new(10, 0)))
                ]),
                gain: Some(Duration::new(38, HALF_SECOND)),
                percent: 88.505745,
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
                duration: Duration::new(25, HALF_SECOND),
                loss: None,
                tasks: FxHashMap::from_iter([(
                    "task".into(),
                    TaskEstimate::new(Duration::new(30, 0))
                )]),
                gain: Some(Duration::new(20, HALF_SECOND)),
                percent: 80.39216,
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
                duration: Duration::new(77, HALF_SECOND),
                loss: None,
                tasks: FxHashMap::from_iter([
                    (
                        "*".into(),
                        TaskEstimate::with_count(Duration::new(35, 0), 0)
                    ),
                    (
                        "build".into(),
                        TaskEstimate::with_count(Duration::new(25, 0), 2)
                    ),
                    (
                        "lint".into(),
                        TaskEstimate::with_count(Duration::new(17, 0), 2)
                    ),
                    ("test".into(), TaskEstimate::new(Duration::new(8, 0)))
                ]),
                gain: Some(Duration::new(52, HALF_SECOND)),
                percent: 67.741936,
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
                duration: Duration::new(77, HALF_SECOND),
                loss: Some(Duration::new(7, HALF_SECOND)),
                tasks: FxHashMap::from_iter([
                    (
                        "*".into(),
                        TaskEstimate::with_count(Duration::new(35, 0), 0)
                    ),
                    (
                        "build".into(),
                        TaskEstimate::with_count(Duration::new(25, 0), 2)
                    ),
                    (
                        "lint".into(),
                        TaskEstimate::with_count(Duration::new(17, 0), 2)
                    ),
                    ("test".into(), TaskEstimate::new(Duration::new(8, 0)))
                ]),
                gain: None,
                percent: -8.823529,
            },
        )
    }
}
