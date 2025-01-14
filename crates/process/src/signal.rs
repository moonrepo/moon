// https://www.math.stonybrook.edu/~ccc/dfc/dfc/signals.html
// https://sunshowers.io/posts/beyond-ctrl-c-signals/

use tokio::sync::broadcast::Sender;
use tracing::debug;

#[derive(Clone, Copy, Debug)]
pub enum SignalType {
    Interrupt,
    Quit,
    Terminate,
}

#[cfg(unix)]
pub async fn wait_for_signal(sender: Sender<SignalType>) {
    use tokio::signal::unix::{signal, SignalKind};

    debug!("Listening for SIGINT, SIGQUIT, and SIGTERM signals");

    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
    let mut signal_quit = signal(SignalKind::quit()).unwrap();

    let _ = tokio::select! {
        _ = signal_terminate.recv() => {
            debug!("Received SIGTERM signal");
            sender.send(SignalType::Terminate)
        },
        _ = signal_interrupt.recv() => {
            debug!("Received SIGINT signal");
            sender.send(SignalType::Interrupt)
        },
        _ = signal_quit.recv() => {
            debug!("Received SIGQUIT signal");
            sender.send(SignalType::Quit)
        },
    };
}

#[cfg(windows)]
pub async fn wait_for_signal(sender: Sender<SignalType>) {
    use tokio::signal::windows;

    debug!("Listening for CTRL-C, BREAK, CLOSE, and SHUTDOWN signals");

    let mut signal_c = windows::ctrl_c().unwrap(); // Interrupt
    let mut signal_break = windows::ctrl_break().unwrap(); // Interrupt
    let mut signal_close = windows::ctrl_close().unwrap(); // Quit
    let mut signal_shutdown = windows::ctrl_shutdown().unwrap(); // Terminate

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
            sender.send(SignalType::Quit)
        },
        _ = signal_shutdown.recv() => {
            debug!("Received CTRL-SHUTDOWN signal");
            sender.send(SignalType::Terminate)
        },
    };
}

#[cfg(unix)]
pub fn kill(pid: u32, signal: SignalType) -> std::io::Result<()> {
    let result = unsafe {
        libc::kill(
            pid as i32,
            match signal {
                SignalType::Interrupt => 2,  // SIGINT
                SignalType::Quit => 3,       // SIGQUIT
                SignalType::Terminate => 15, // SIGTERM
            },
        )
    };

    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}
