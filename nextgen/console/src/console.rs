use miette::IntoDiagnostic;
use parking_lot::RwLock;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum ConsoleStream {
    Stderr,
    Stdout,
}

pub struct ConsoleBuffer {
    buffer: Arc<RwLock<BufWriter<Vec<u8>>>>,
    closed: bool,
    channel: Option<Sender<bool>>,
    handle: Option<JoinHandle<()>>,
    stream: ConsoleStream,

    pub quiet: bool,
    pub test_mode: bool,
}

impl ConsoleBuffer {
    fn internal_new(stream: ConsoleStream, with_handle: bool) -> Self {
        let buffer = Arc::new(RwLock::new(BufWriter::new(Vec::new())));
        let buffer_clone = Arc::clone(&buffer);
        let (tx, rx) = mpsc::channel();

        // Every 100ms, flush the buffer
        let handle = if with_handle {
            Some(spawn(move || flush_on_loop(buffer_clone, stream, rx)))
        } else {
            None
        };

        Self {
            buffer,
            closed: false,
            channel: Some(tx),
            handle,
            stream,
            quiet: false,
            test_mode: false,
        }
    }

    pub fn new(stream: ConsoleStream, quiet: bool) -> Self {
        let mut console = Self::internal_new(stream, true);
        console.quiet = quiet;
        console
    }

    pub fn new_testing(stream: ConsoleStream) -> Self {
        let mut console = Self::internal_new(stream, false);
        console.test_mode = true;
        console
    }

    pub fn is_terminal(&self) -> bool {
        match self.stream {
            ConsoleStream::Stderr => io::stderr().is_terminal(),
            ConsoleStream::Stdout => io::stdout().is_terminal(),
        }
    }

    pub fn close(&mut self) -> miette::Result<()> {
        self.flush()?;

        self.closed = true;

        // Send the closed message
        if let Some(channel) = self.channel.take() {
            let _ = channel.send(true);
        }

        // Attempt to close the thread
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    pub fn flush(&self) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

        flush(&mut self.buffer.write(), self.stream).into_diagnostic()?;

        Ok(())
    }

    pub fn write_raw<F: FnMut(&mut BufWriter<Vec<u8>>) -> io::Result<()>>(
        &self,
        mut op: F,
    ) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

        // When testing just flush immediately
        if self.test_mode {
            let mut buffer = BufWriter::new(Vec::new());

            op(&mut buffer).into_diagnostic()?;

            drain(&mut buffer, self.stream).into_diagnostic()?;

            return Ok(());
        }

        // Otherwise just write to the buffer
        let mut buffer = self.buffer.write();

        op(&mut buffer).into_diagnostic()?;

        // Buffer has written its data to the inner vec, so drain it
        if !buffer.get_ref().is_empty() {
            drain(&mut buffer, self.stream).into_diagnostic()?;
        }

        Ok(())
    }

    pub fn write<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let data = data.as_ref();

        if data.is_empty() {
            return Ok(());
        }

        self.write_raw(|buffer| buffer.write_all(data))
    }

    pub fn write_line<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let data = data.as_ref();

        if data.is_empty() {
            return Ok(());
        }

        self.write_raw(|buffer| {
            buffer.write_all(data)?;
            buffer.write_all(b"\n")?;
            Ok(())
        })
    }

    pub fn write_newline(&self) -> miette::Result<()> {
        self.write("\n")
    }
}

impl Drop for ConsoleBuffer {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

impl Clone for ConsoleBuffer {
    fn clone(&self) -> Self {
        Self {
            buffer: Arc::clone(&self.buffer),
            closed: self.closed,
            stream: self.stream,
            quiet: self.quiet,
            test_mode: self.test_mode,
            // Ignore for clones
            channel: None,
            handle: None,
        }
    }
}

#[derive(Clone)]
pub struct Console {
    pub err: ConsoleBuffer,
    pub out: ConsoleBuffer,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        Self {
            err: ConsoleBuffer::new(ConsoleStream::Stderr, quiet),
            out: ConsoleBuffer::new(ConsoleStream::Stdout, quiet),
        }
    }

    pub fn new_testing() -> Self {
        Self {
            err: ConsoleBuffer::new_testing(ConsoleStream::Stderr),
            out: ConsoleBuffer::new_testing(ConsoleStream::Stdout),
        }
    }

    // This should be safe since there will only be one console instance
    pub fn close(&mut self) -> miette::Result<()> {
        self.err.close()?;
        self.out.close()?;

        Ok(())
    }

    pub fn stderr(&self) -> &ConsoleBuffer {
        &self.err
    }

    pub fn stdout(&self) -> &ConsoleBuffer {
        &self.out
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

fn drain(buffer: &mut BufWriter<Vec<u8>>, stream: ConsoleStream) -> io::Result<()> {
    let data = buffer.get_mut().drain(0..).collect::<Vec<_>>();

    match stream {
        ConsoleStream::Stderr => io::stderr().lock().write_all(&data),
        ConsoleStream::Stdout => io::stdout().lock().write_all(&data),
    }
}

fn flush(buffer: &mut BufWriter<Vec<u8>>, stream: ConsoleStream) -> io::Result<()> {
    buffer.flush()?;

    if buffer.get_ref().is_empty() {
        return Ok(());
    }

    drain(buffer, stream)
}

fn flush_on_loop(
    buffer: Arc<RwLock<BufWriter<Vec<u8>>>>,
    stream: ConsoleStream,
    receiver: Receiver<bool>,
) {
    loop {
        sleep(Duration::from_millis(100));

        let _ = flush(&mut buffer.write(), stream);

        // Has the thread been closed?
        match receiver.try_recv() {
            Ok(true) | Err(TryRecvError::Disconnected) => {
                break;
            }
            _ => {}
        }
    }
}
