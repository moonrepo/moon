#![cfg(unix)]

use moon_process::{ChildExit, SharedChild, SignalType};
use std::process::Stdio;
use tokio::process::Command;

fn spawn_sleep() -> SharedChild {
    SharedChild::new(Command::new("sleep").arg("30").spawn().unwrap())
}

mod shared_child {
    use super::*;

    #[tokio::test]
    async fn returns_a_pid() {
        let child = spawn_sleep();

        assert!(child.id() > 0);

        let _ = child.kill().await;
    }

    #[tokio::test]
    async fn takes_pipes_only_once() {
        let mut command = Command::new("sleep");
        command.arg("30").stdout(Stdio::piped());

        let child = SharedChild::new(command.spawn().unwrap());

        assert!(child.take_stdout().await.is_some());
        assert!(child.take_stdout().await.is_none());
        assert!(child.take_stderr().await.is_none());

        let _ = child.kill().await;
    }

    #[tokio::test]
    async fn kill_reports_killed() {
        assert_eq!(spawn_sleep().kill().await.unwrap(), ChildExit::Killed);
    }

    #[tokio::test]
    async fn interrupt_signal_reports_interrupted() {
        assert_eq!(
            spawn_sleep()
                .kill_with_signal(SignalType::Interrupt)
                .await
                .unwrap(),
            ChildExit::Interrupted
        );
    }

    #[tokio::test]
    async fn kill_signal_reports_killed() {
        assert_eq!(
            spawn_sleep()
                .kill_with_signal(SignalType::Kill)
                .await
                .unwrap(),
            ChildExit::Killed
        );
    }

    #[tokio::test]
    async fn terminate_signal_reports_terminated() {
        assert_eq!(
            spawn_sleep()
                .kill_with_signal(SignalType::Terminate)
                .await
                .unwrap(),
            ChildExit::Terminated
        );
    }
}
