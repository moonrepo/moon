use miette::IntoDiagnostic;
use parking_lot::Mutex;
use std::io::{self, IsTerminal, Write};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
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
    buffer: Arc<Mutex<Vec<u8>>>,
    channel: Option<Sender<bool>>,
    stream: ConsoleStream,

    pub(crate) handle: Option<JoinHandle<()>>,
    pub(crate) quiet: Option<Arc<AtomicBool>>,
    pub(crate) test_mode: bool,
}

impl ConsoleBuffer {
    fn internal_new(stream: ConsoleStream, with_handle: bool) -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
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
            channel: Some(tx),
            handle,
            stream,
            quiet: None,
            test_mode: false,
        }
    }

    pub fn new(stream: ConsoleStream) -> Self {
        Self::internal_new(stream, true)
    }

    pub fn new_testing(stream: ConsoleStream) -> Self {
        let mut console = Self::internal_new(stream, false);
        console.test_mode = true;
        console
    }

    pub fn empty(stream: ConsoleStream) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            channel: None,
            stream,
            handle: None,
            quiet: None,
            test_mode: false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self.stream {
            ConsoleStream::Stderr => io::stderr().is_terminal(),
            ConsoleStream::Stdout => io::stdout().is_terminal(),
        }
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
            .as_ref()
            .is_some_and(|quiet| quiet.load(Ordering::Relaxed))
    }

    pub fn close(&self) -> miette::Result<()> {
        self.flush()?;

        // Send the closed message
        if let Some(channel) = &self.channel {
            let _ = channel.send(true);
        }

        Ok(())
    }

    pub fn flush(&self) -> miette::Result<()> {
        flush(&mut self.buffer.lock(), self.stream).into_diagnostic()?;

        Ok(())
    }

    pub fn write_raw<F: FnMut(&mut Vec<u8>)>(&self, mut op: F) -> miette::Result<()> {
        // When testing just flush immediately
        if self.test_mode {
            let mut buffer = Vec::new();

            op(&mut buffer);

            flush(&mut buffer, self.stream).into_diagnostic()?;
        }
        // Otherwise just write to the buffer and flush
        // when its length grows too large
        else {
            let mut buffer = self.buffer.lock();

            op(&mut buffer);

            if buffer.len() >= 1024 {
                flush(&mut buffer, self.stream).into_diagnostic()?;
            }
        }

        Ok(())
    }

    pub fn write<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let data = data.as_ref();

        if data.is_empty() {
            return Ok(());
        }

        self.write_raw(|buffer| buffer.extend_from_slice(data))
    }

    pub fn write_line<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let data = data.as_ref();

        if data.is_empty() {
            return Ok(());
        }

        self.write_raw(|buffer| {
            buffer.extend_from_slice(data);
            buffer.push(b'\n');
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
            stream: self.stream,
            quiet: self.quiet.clone(),
            test_mode: self.test_mode,
            // Ignore for clones
            channel: None,
            handle: None,
        }
    }
}

fn flush(buffer: &mut Vec<u8>, stream: ConsoleStream) -> io::Result<()> {
    if buffer.is_empty() {
        return Ok(());
    }

    let data = mem::take(buffer);

    match stream {
        ConsoleStream::Stderr => io::stderr().lock().write_all(&data),
        ConsoleStream::Stdout => io::stdout().lock().write_all(&data),
    }
}

fn flush_on_loop(buffer: Arc<Mutex<Vec<u8>>>, stream: ConsoleStream, receiver: Receiver<bool>) {
    loop {
        sleep(Duration::from_millis(100));

        let _ = flush(&mut buffer.lock(), stream);

        // Has the thread been closed?
        match receiver.try_recv() {
            Ok(true) | Err(TryRecvError::Disconnected) => {
                break;
            }
            _ => {}
        }
    }
}
