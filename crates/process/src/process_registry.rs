use crate::shared_child::*;
use crate::signal::*;
use core::time::Duration;
use rustc_hash::FxHashMap;
use std::sync::{Arc, OnceLock};
use tokio::process::Child;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, trace};

static INSTANCE: OnceLock<Arc<ProcessRegistry>> = OnceLock::new();

pub struct ProcessRegistry {
    processes: Arc<RwLock<FxHashMap<u32, SharedChild>>>,
    signal_sender: Sender<SignalType>,
    signal_wait_handle: JoinHandle<()>,
    signal_shutdown_handle: JoinHandle<()>,
}

impl ProcessRegistry {
    pub fn instance() -> Arc<ProcessRegistry> {
        Arc::clone(INSTANCE.get_or_init(|| Arc::new(ProcessRegistry::new())))
    }

    pub fn new() -> Self {
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
            processes,
            signal_sender: sender,
            signal_wait_handle,
            signal_shutdown_handle,
        }
    }

    pub async fn add_child(&self, child: Child) -> SharedChild {
        let shared = SharedChild::new(child);

        self.processes
            .write()
            .await
            .insert(shared.id(), shared.clone());

        shared
    }

    pub async fn get_child_by_id(&self, id: u32) -> Option<SharedChild> {
        self.processes.read().await.get(&id).cloned()
    }

    pub async fn remove_child(&self, child: SharedChild) {
        self.remove_child_by_id(child.id()).await
    }

    pub async fn remove_child_by_id(&self, id: u32) {
        self.processes.write().await.remove(&id);
    }

    pub fn receive_signal(&self) -> Receiver<SignalType> {
        self.signal_sender.subscribe()
    }

    pub fn terminate_children(&self) {
        let _ = self.signal_sender.send(SignalType::Terminate);
    }

    pub async fn wait_for_children_to_shutdown(&self) {
        let mut count = 0;

        loop {
            if self.processes.read().await.is_empty() || count >= 5000 {
                break;
            }

            sleep(Duration::from_millis(50)).await;
            count += 50;
        }
    }
}

impl Drop for ProcessRegistry {
    fn drop(&mut self) {
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

        let _ = child.kill_with_signal(signal).await;

        drop(child);
    }
}

#[cfg(unix)]
async fn wait_for_signal(sender: Sender<SignalType>) {
    debug!("Listening for SIGINT and SIGTERM signals");

    use tokio::signal::unix::{signal, SignalKind};

    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();

    let _ = tokio::select! {
        _ = signal_terminate.recv() => {
            debug!("Received SIGTERM signal");
            sender.send(SignalType::Terminate)
        },
        _ = signal_interrupt.recv() => {
            debug!("Received SIGINT signal");
            sender.send(SignalType::Interrupt)
        },
    };
}

#[cfg(windows)]
async fn wait_for_signal(sender: Sender<SignalType>) {
    debug!("Listening for CTRL-C, BREAK, CLOSE, and SHUTDOWN signals");

    use tokio::signal::windows;

    let mut signal_c = windows::ctrl_c().unwrap();
    let mut signal_break = windows::ctrl_break().unwrap();
    let mut signal_close = windows::ctrl_close().unwrap();
    let mut signal_shutdown = windows::ctrl_shutdown().unwrap();

    let _ = tokio::select! {
        _ = signal_c.recv() => => {
            debug!("Received CTRL-C signal");
            sender.send(SignalType::Interrupt)
        },
        _ = signal_break.recv() => => {
            debug!("Received CTRL-BREAK signal");
            sender.send(SignalType::Interrupt)
        },
        _ = signal_close.recv() => => {
            debug!("Received CTRL-CLOSE signal");
            sender.send(SignalType::Interrupt)
        },
        _ = signal_shutdown.recv() => {
            debug!("Received CTRL-SHUTDOWN signal");
            sender.send(SignalType::Terminate)
        },
    };
}
