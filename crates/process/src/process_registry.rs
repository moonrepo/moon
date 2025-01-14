use crate::shared_child::*;
use crate::signal::*;
use core::time::Duration;
use rustc_hash::FxHashMap;
use std::sync::{Arc, OnceLock};
use tokio::process::Child;
use tokio::sync::broadcast::{self, error::RecvError, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, trace, warn};

static INSTANCE: OnceLock<Arc<ProcessRegistry>> = OnceLock::new();

pub struct ProcessRegistry {
    running: Arc<RwLock<FxHashMap<u32, SharedChild>>>,
    signal_sender: Sender<SignalType>,
    signal_wait_handle: JoinHandle<()>,
    signal_shutdown_handle: JoinHandle<()>,
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        let processes = Arc::new(RwLock::new(FxHashMap::default()));
        let processes_bg = Arc::clone(&processes);

        let (sender, receiver) = broadcast::channel::<SignalType>(10);
        let sender_bg = sender.clone();

        let signal_wait_handle = tokio::spawn(async move {
            wait_for_signal(sender_bg).await;
        });

        let signal_shutdown_handle = tokio::spawn(async move {
            shutdown_processes_with_signal(receiver, processes_bg).await;
        });

        Self {
            running: processes,
            signal_sender: sender,
            signal_wait_handle,
            signal_shutdown_handle,
        }
    }
}

impl ProcessRegistry {
    pub fn instance() -> Arc<ProcessRegistry> {
        Arc::clone(INSTANCE.get_or_init(|| Arc::new(ProcessRegistry::default())))
    }

    pub async fn add_running(&self, child: Child) -> SharedChild {
        let shared = SharedChild::new(child);

        self.running
            .write()
            .await
            .insert(shared.id(), shared.clone());

        shared
    }

    pub async fn get_running_by_pid(&self, id: u32) -> Option<SharedChild> {
        self.running.read().await.get(&id).cloned()
    }

    pub async fn remove_running(&self, child: SharedChild) {
        self.remove_running_by_pid(child.id()).await
    }

    pub async fn remove_running_by_pid(&self, id: u32) {
        self.running.write().await.remove(&id);
    }

    pub fn receive_signal(&self) -> Receiver<SignalType> {
        self.signal_sender.subscribe()
    }

    pub fn terminate_running(&self) {
        let _ = self.signal_sender.send(SignalType::Terminate);
    }

    pub async fn wait_for_running_to_shutdown(&self) {
        let mut count = 0;

        loop {
            // Wait for all running processes to have stopped,
            // or if we have waited 5 seconds, just quit
            if self.running.read().await.is_empty() || count >= 5000 {
                break;
            }

            sleep(Duration::from_millis(50)).await;
            count += 50;
        }
    }
}

impl Drop for ProcessRegistry {
    fn drop(&mut self) {
        self.terminate_running();
        self.signal_wait_handle.abort();
        self.signal_shutdown_handle.abort();
    }
}

async fn shutdown_processes_with_signal(
    mut receiver: Receiver<SignalType>,
    processes: Arc<RwLock<FxHashMap<u32, SharedChild>>>,
) {
    let signal: SignalType;

    loop {
        signal = match receiver.recv().await {
            Ok(signal) => signal,
            Err(RecvError::Closed) => SignalType::Terminate,
            _ => continue,
        };

        break;
    }

    let mut children = processes.write().await;

    if children.is_empty() {
        return;
    }

    debug!(
        pids = ?children.keys().collect::<Vec<_>>(),
        signal = ?signal,
        "Shutting down {} running child processes",
        children.len()
    );

    for (pid, child) in children.drain() {
        trace!(pid, "Killing child process");

        if let Err(error) = child.kill_with_signal(signal).await {
            warn!(pid, "Failed to kill child process: {error}");
        }

        drop(child);
    }
}
