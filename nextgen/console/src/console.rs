use miette::IntoDiagnostic;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum ConsoleStream {
    Stderr,
    Stdout,
}

pub struct Console {
    buffer: Arc<RwLock<BufWriter<Vec<u8>>>>,
    closed: bool,
    channel: Sender<bool>,
    handle: Option<JoinHandle<()>>,
    stream: ConsoleStream,

    pub quiet: bool,
}

impl Console {
    pub fn new(target: ConsoleStream, quiet: bool) -> Self {
        let buffer = Arc::new(RwLock::new(BufWriter::new(Vec::new())));
        let buffer_clone = Arc::clone(&buffer);
        let (tx, rx) = mpsc::channel();

        // Every 100ms, flush the buffer
        let handle = spawn(move || loop {
            sleep(Duration::from_millis(100));

            if let Ok(mut out) = buffer_clone.write() {
                let _ = flush(&mut out, target);
            } else {
                break;
            }

            // Has the thread been closed?
            match rx.try_recv() {
                Ok(true) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                _ => {}
            }
        });

        Self {
            buffer,
            closed: false,
            channel: tx,
            handle: Some(handle),
            stream: target,
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
        let _ = self.handle.take().unwrap().join();

        self.closed = true;

        Ok(())
    }

    pub fn flush(&self) -> miette::Result<()> {
        if let Ok(mut out) = self.buffer.write() {
            flush(&mut out, self.stream).into_diagnostic()?;
        }

        Ok(())
    }

    pub fn write<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        if self.closed {
            return Ok(());
        }

        let mut buffer = self
            .buffer
            .write()
            .expect("Failed to acquire console write lock.");

        buffer.write_all(data.as_ref()).into_diagnostic()?;

        // Buffer has written its data to the inner vec, so flush it
        if !buffer.get_ref().is_empty() {
            drain(&mut buffer, self.stream).into_diagnostic()?;
        }

        Ok(())
    }

    pub fn write_line<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let mut data = data.as_ref().to_owned();
        data.push(b'\n');

        self.write(data)
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
