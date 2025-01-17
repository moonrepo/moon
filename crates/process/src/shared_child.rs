use crate::signal::*;
use process_wrap::tokio::TokioChildWrapper;
use std::io;
use std::process::{ExitStatus, Output};
use std::sync::Arc;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SharedChild {
    inner: Arc<Mutex<Box<dyn TokioChildWrapper>>>,
    pid: u32,
}

impl SharedChild {
    pub fn new(child: Box<dyn TokioChildWrapper>) -> Self {
        Self {
            pid: child.id().unwrap(),
            inner: Arc::new(Mutex::new(child)),
        }
    }

    pub fn id(&self) -> u32 {
        self.pid
    }

    pub async fn take_stdin(&self) -> Option<ChildStdin> {
        self.inner.lock().await.as_mut().stdin().take()
    }

    pub async fn take_stdout(&self) -> Option<ChildStdout> {
        self.inner.lock().await.as_mut().stdout().take()
    }

    pub async fn take_stderr(&self) -> Option<ChildStderr> {
        self.inner.lock().await.as_mut().stderr().take()
    }

    pub async fn kill(&self) -> io::Result<()> {
        let mut child = self.inner.lock().await;

        Box::into_pin(child.kill()).await?;

        Ok(())
    }

    pub async fn kill_with_signal(&self, signal: SignalType) -> io::Result<()> {
        let mut child = self.inner.lock().await;

        dbg!("kill_with_signal", self.id(), signal);

        // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/unix/process/process_unix.rs#L947
        #[cfg(unix)]
        {
            child.signal(match signal {
                SignalType::Interrupt => 2,  // SIGINT
                SignalType::Quit => 3,       // SIGQUIT
                SignalType::Terminate => 15, // SIGTERM
            })?;
        }

        // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/windows/process.rs#L658
        #[cfg(windows)]
        {
            child.start_kill()?;
        }

        Box::into_pin(child.wait()).await?;

        Ok(())
    }

    pub(crate) async fn wait(&self) -> io::Result<ExitStatus> {
        let mut child = self.inner.lock().await;

        Box::into_pin(child.wait()).await
    }

    // This method re-implements the tokio `wait_with_output` method
    // but does not take ownership of self. This is required to be able
    // to call `kill`, otherwise the child does not exist.
    pub(crate) async fn wait_with_output(&self) -> io::Result<Output> {
        use tokio::{io::AsyncReadExt, try_join};

        let mut child = self.inner.lock().await;

        async fn read_to_end<A: AsyncReadExt + Unpin>(data: &mut Option<A>) -> io::Result<Vec<u8>> {
            let mut vec = Vec::new();
            if let Some(data) = data.as_mut() {
                data.read_to_end(&mut vec).await?;
            }
            Ok(vec)
        }

        let mut stdout_pipe = child.stdout().take();
        let mut stderr_pipe = child.stderr().take();

        let stdout_fut = read_to_end(&mut stdout_pipe);
        let stderr_fut = read_to_end(&mut stderr_pipe);

        let (status, stdout, stderr) =
            try_join!(Box::into_pin(child.wait()), stdout_fut, stderr_fut)?;

        drop(stdout_pipe);
        drop(stderr_pipe);

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}
