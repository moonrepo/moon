use crate::prompts::create_theme;
use inquire::ui::RenderConfig;
use miette::IntoDiagnostic;
use moon_common::is_formatted_output;
use parking_lot::Mutex;
use starbase::Resource;
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
    closed: bool,
    channel: Option<Sender<bool>>,
    handle: Option<JoinHandle<()>>,
    stream: ConsoleStream,

    quiet: Option<Arc<AtomicBool>>,
    test_mode: bool,
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
            closed: false,
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

        flush(&mut self.buffer.lock(), self.stream).into_diagnostic()?;

        Ok(())
    }

    pub fn write_raw<F: FnMut(&mut Vec<u8>)>(&self, mut op: F) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

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
            closed: self.closed,
            stream: self.stream,
            quiet: self.quiet.clone(),
            test_mode: self.test_mode,
            // Ignore for clones
            channel: None,
            handle: None,
        }
    }
}

#[derive(Clone, Resource)]
pub struct Console {
    pub err: ConsoleBuffer,
    pub out: ConsoleBuffer,

    quiet: Arc<AtomicBool>,
    theme: Arc<RenderConfig<'static>>,
}

impl Console {
    pub fn new(quiet: bool) -> Self {
        let quiet = Arc::new(AtomicBool::new(quiet || is_formatted_output()));

        let mut err = ConsoleBuffer::new(ConsoleStream::Stderr);
        err.quiet = Some(Arc::clone(&quiet));

        let mut out = ConsoleBuffer::new(ConsoleStream::Stdout);
        out.quiet = Some(Arc::clone(&quiet));

        Self {
            err,
            out,
            quiet,
            theme: Arc::new(create_theme()),
        }
    }

    pub fn new_testing() -> Self {
        Self {
            err: ConsoleBuffer::new_testing(ConsoleStream::Stderr),
            out: ConsoleBuffer::new_testing(ConsoleStream::Stdout),
            quiet: Arc::new(AtomicBool::new(false)),
            theme: Arc::new(RenderConfig::empty()),
        }
    }

    pub fn close(&mut self) -> miette::Result<()> {
        self.err.close()?;
        self.out.close()?;

        Ok(())
    }

    pub fn quiet(&self) {
        self.quiet.store(true, Ordering::Release);
    }

    pub fn stderr(&self) -> &ConsoleBuffer {
        &self.err
    }

    pub fn stdout(&self) -> &ConsoleBuffer {
        &self.out
    }

    pub fn theme(&self) -> Arc<RenderConfig<'static>> {
        Arc::clone(&self.theme)
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        self.close().unwrap();
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
