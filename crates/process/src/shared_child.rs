use crate::signal::*;
use std::io;
use std::process::{ExitStatus, Output};
use std::sync::Arc;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SharedChild {
    inner: Arc<Mutex<Child>>,
    pid: u32,
}

impl SharedChild {
    pub fn new(child: Child) -> Self {
        Self {
            pid: child.id().unwrap(),
            inner: Arc::new(Mutex::new(child)),
        }
    }

    pub fn id(&self) -> u32 {
        self.pid
    }

    pub async fn take_stdin(&self) -> Option<ChildStdin> {
        self.inner.lock().await.stdin.take()
    }

    pub async fn take_stdout(&self) -> Option<ChildStdout> {
        self.inner.lock().await.stdout.take()
    }

    pub async fn take_stderr(&self) -> Option<ChildStderr> {
        self.inner.lock().await.stderr.take()
    }

    pub async fn kill(&self) -> io::Result<()> {
        let mut child = self.inner.lock().await;

        child.kill().await?;

        Ok(())
    }

    pub async fn kill_with_signal(&self, signal: SignalType) -> io::Result<()> {
        let mut child = self.inner.lock().await;

        dbg!("kill_with_signal", self.id(), signal);

        // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/unix/process/process_unix.rs#L947
        #[cfg(unix)]
        {
            kill(self.id(), signal)?;
        }

        // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/windows/process.rs#L658
        #[cfg(windows)]
        {
            child.start_kill()?;
        }

        child.wait().await?;

        Ok(())
    }

    pub(crate) async fn wait(&self) -> io::Result<ExitStatus> {
        let mut child = self.inner.lock().await;

        child.wait().await
    }

    // This method re-implements the tokio `wait_with_output` method
    // but does not take ownership of self. This is required to be able
    // to call `kill`, otherwise the child does not exist.
    pub(crate) async fn wait_with_output(&self) -> io::Result<Output> {
        use tokio::{io::AsyncReadExt, try_join};

        async fn read_to_end<A: AsyncReadExt + Unpin>(data: &mut Option<A>) -> io::Result<Vec<u8>> {
            let mut vec = Vec::new();

            if let Some(data) = data.as_mut() {
                data.read_to_end(&mut vec).await?;
            }

            Ok(vec)
        }

        let mut child = self.inner.lock().await;
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        let stdout_fut = read_to_end(&mut stdout_pipe);
        let stderr_fut = read_to_end(&mut stderr_pipe);

        let (status, stdout, stderr) = try_join!(child.wait(), stdout_fut, stderr_fut)?;

        drop(stdout_pipe);
        drop(stderr_pipe);

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}
