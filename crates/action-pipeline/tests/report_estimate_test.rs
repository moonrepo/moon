use moon_action::*;
use moon_action_pipeline::reports::estimate::{Estimate, TaskEstimate};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_toolchain::Runtime;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::Duration;

const NANOS_PER_MILLI: u32 = 1_000_000;
const HALF_SECOND: u32 = NANOS_PER_MILLI * 500;

fn create_run_task_action(runtime: Runtime, target: &str) -> Arc<ActionNode> {
    Arc::new(ActionNode::run_task(RunTaskNode::new(
        target.into(),
        runtime,
    )))
}

mod estimate {
    use super::*;

    #[test]
    fn returns_loss_state() {
        let est = Estimate::calculate(&[], &Duration::new(5, 0));

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[Action {
                duration: Some(Duration::new(10, 0)),
                node: create_run_task_action(Runtime::system(), "proj:task"),
                ..Action::default()
            }],
            &Duration::new(5, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: create_run_task_action(Runtime::system(), "a:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(5, 0)),
                    node: create_run_task_action(Runtime::system(), "a:lint"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(15, 0)),
                    node: create_run_task_action(Runtime::system(), "b:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(8, 0)),
                    node: create_run_task_action(Runtime::system(), "c:test"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(12, 0)),
                    node: create_run_task_action(Runtime::system(), "d:lint"),
                    ..Action::default()
                },
            ],
            &Duration::new(10, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: Arc::new(ActionNode::setup_toolchain_legacy(
                        SetupToolchainLegacyNode {
                            runtime: Runtime::system(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(25, 0)),
                    node: Arc::new(ActionNode::install_workspace_deps(
                        InstallWorkspaceDepsNode {
                            runtime: Runtime::system(),
                            root: WorkspaceRelativePathBuf::new(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: create_run_task_action(Runtime::system(), "proj:task"),
                    ..Action::default()
                },
            ],
            &Duration::new(5, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[Action {
                duration: Some(Duration::new(3, 0)),
                node: create_run_task_action(Runtime::system(), "proj:task"),
                status: ActionStatus::Cached,
                ..Action::default()
            }],
            &Duration::new(5, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: Arc::new(ActionNode::setup_toolchain_legacy(
                        SetupToolchainLegacyNode {
                            runtime: Runtime::system(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(25, 0)),
                    node: Arc::new(ActionNode::install_workspace_deps(
                        InstallWorkspaceDepsNode {
                            runtime: Runtime::system(),
                            root: WorkspaceRelativePathBuf::new(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: create_run_task_action(Runtime::system(), "a:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(5, 0)),
                    node: create_run_task_action(Runtime::system(), "a:lint"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(15, 0)),
                    node: create_run_task_action(Runtime::system(), "b:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(8, 0)),
                    node: create_run_task_action(Runtime::system(), "c:test"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(12, 0)),
                    node: create_run_task_action(Runtime::system(), "d:lint"),
                    ..Action::default()
                },
            ],
            &Duration::new(25, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
        let est = Estimate::calculate(
            &[
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: Arc::new(ActionNode::setup_toolchain_legacy(
                        SetupToolchainLegacyNode {
                            runtime: Runtime::system(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(25, 0)),
                    node: Arc::new(ActionNode::install_workspace_deps(
                        InstallWorkspaceDepsNode {
                            runtime: Runtime::system(),
                            root: WorkspaceRelativePathBuf::new(),
                        },
                    )),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(10, 0)),
                    node: create_run_task_action(Runtime::system(), "a:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(5, 0)),
                    node: create_run_task_action(Runtime::system(), "a:lint"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(15, 0)),
                    node: create_run_task_action(Runtime::system(), "b:build"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(8, 0)),
                    node: create_run_task_action(Runtime::system(), "c:test"),
                    ..Action::default()
                },
                Action {
                    duration: Some(Duration::new(12, 0)),
                    node: create_run_task_action(Runtime::system(), "d:lint"),
                    ..Action::default()
                },
            ],
            &Duration::new(85, 0),
        );

        assert_eq!(
            est,
            Estimate {
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
