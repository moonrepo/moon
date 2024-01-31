use miette::IntoDiagnostic;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
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
    channel: Sender<bool>,
    handle: Option<JoinHandle<()>>,
    stream: ConsoleStream,

    pub quiet: bool,
}

impl ConsoleBuffer {
    pub fn new(stream: ConsoleStream, quiet: bool) -> Self {
        let buffer = Arc::new(RwLock::new(BufWriter::new(Vec::new())));
        let buffer_clone = Arc::clone(&buffer);
        let (tx, rx) = mpsc::channel();

        // Every 100ms, flush the buffer
        let handle = spawn(move || flush_on_loop(buffer_clone, stream, rx));

        Self {
            buffer,
            closed: false,
            channel: tx,
            handle: Some(handle),
            stream,
            quiet,
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self.stream {
            ConsoleStream::Stderr => io::stderr().is_terminal(),
            ConsoleStream::Stdout => io::stdout().is_terminal(),
        }
    }

    pub fn close(&mut self) -> miette::Result<()> {
        self.flush()?;

        // Send the closed message
        let _ = self.channel.send(true);

        // Attempt to close the thread
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        self.closed = true;

        Ok(())
    }

    pub fn flush(&self) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

        if let Ok(mut out) = self.buffer.write() {
            flush(&mut out, self.stream).into_diagnostic()?;
        }

        Ok(())
    }

    pub fn write_raw<F: FnMut(&mut BufWriter<Vec<u8>>) -> io::Result<()>>(
        &self,
        mut op: F,
    ) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

        let mut buffer = self
            .buffer
            .write()
            .expect("Failed to acquire console write lock.");

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
            channel: self.channel.clone(),
            handle: None, // Ignore for clones
            stream: self.stream,
            quiet: self.quiet,
        }
    }
}

#[derive(Clone)]
pub struct Console {
    pub err: Arc<ConsoleBuffer>,
    pub out: Arc<ConsoleBuffer>,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        Self {
            err: Arc::new(ConsoleBuffer::new(ConsoleStream::Stderr, quiet)),
            out: Arc::new(ConsoleBuffer::new(ConsoleStream::Stdout, quiet)),
        }
    }

    // This should be safe since there will only be one console instance
    pub fn close(&mut self) -> miette::Result<()> {
        if let Some(err) = Arc::get_mut(&mut self.err) {
            err.close()?
        }

        if let Some(out) = Arc::get_mut(&mut self.out) {
            out.close()?;
        }

        Ok(())
    }

    pub fn stderr(&self) -> Arc<ConsoleBuffer> {
        Arc::clone(&self.err)
    }

    pub fn stdout(&self) -> Arc<ConsoleBuffer> {
        Arc::clone(&self.out)
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

        if let Ok(mut out) = buffer.write() {
            let _ = flush(&mut out, stream);
        } else {
            break;
        }

        // Has the thread been closed?
        match receiver.try_recv() {
            Ok(true) | Err(TryRecvError::Disconnected) => {
                break;
            }
            _ => {}
        }
    }
}
