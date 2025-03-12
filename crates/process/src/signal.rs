// https://www.math.stonybrook.edu/~ccc/dfc/dfc/signals.html
// https://sunshowers.io/posts/beyond-ctrl-c-signals/

use std::io;
use tokio::sync::broadcast::Sender;
use tracing::debug;

#[derive(Clone, Copy, Debug)]
pub enum SignalType {
    Interrupt,
    Kill,
    Quit,
    Terminate,
}

#[cfg(unix)]
mod unix {
    use super::*;

    pub async fn wait_for_signal(sender: Sender<SignalType>) {
        use tokio::signal::unix::{SignalKind, signal};

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

    pub fn kill(pid: u32, signal: SignalType) -> io::Result<()> {
        let result = unsafe {
            libc::kill(
                pid as i32,
                match signal {
                    SignalType::Interrupt => 2,  // SIGINT
                    SignalType::Quit => 3,       // SIGQUIT
                    SignalType::Kill => 9,       // SIGKILL
                    SignalType::Terminate => 15, // SIGTERM
                },
            )
        };

        if result != 0 {
            let error = io::Error::last_os_error();

            // "No such process" error, so it may have been killed already
            // https://man7.org/linux/man-pages/man3/errno.3.html
            if error.raw_os_error().is_some_and(|code| code == 3) {
                return Ok(());
            }

            return Err(error);
        }

        Ok(())
    }
}

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::raw::c_void;
    // use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};
    use windows_sys::Win32::System::Threading::TerminateProcess;

    pub async fn wait_for_signal(sender: Sender<SignalType>) {
        use tokio::signal::windows;

        debug!("Listening for CTRL-C, BREAK, CLOSE, and SHUTDOWN signals");

        let mut signal_c = windows::ctrl_c().unwrap();
        let mut signal_break = windows::ctrl_break().unwrap();
        let mut signal_close = windows::ctrl_close().unwrap();
        let mut signal_shutdown = windows::ctrl_shutdown().unwrap();

        let _ = tokio::select! {
            _ = signal_c.recv() => {
                debug!("Received CTRL-C signal");
                sender.send(SignalType::Interrupt)
            },
            _ = signal_break.recv() => {
                debug!("Received CTRL-BREAK signal");
                sender.send(SignalType::Interrupt)
            },
            _ = signal_close.recv() => {
                debug!("Received CTRL-CLOSE signal");
                sender.send(SignalType::Quit)
            },
            _ = signal_shutdown.recv() => {
                debug!("Received CTRL-SHUTDOWN signal");
                sender.send(SignalType::Terminate)
            },
        };
    }

    #[derive(Clone)]
    pub struct RawHandle(pub *mut c_void);

    unsafe impl Send for RawHandle {}
    unsafe impl Sync for RawHandle {}

    pub fn kill(_pid: u32, handle: RawHandle, signal: SignalType) -> io::Result<()> {
        let result = match signal {
            // https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent
            SignalType::Interrupt => {
                // Do nothing and let signals pass through natively!
                // unsafe {
                //     GenerateConsoleCtrlEvent(
                //         // We can't use CTRL_C_EVENT here, as it doesn't propagate
                //         CTRL_BREAK_EVENT,
                //         pid,
                //     )
                // }
                1
            }
            // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess
            _ => unsafe { TerminateProcess(handle.0, 1) },
        };

        if result == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}

#[cfg(windows)]
pub use windows::*;
