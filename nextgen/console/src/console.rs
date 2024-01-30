use std::io::{self, BufWriter, IsTerminal, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum ConsoleTarget {
    Stderr,
    Stdout,
}

pub struct Console {
    buffer: Arc<RwLock<BufWriter<Vec<u8>>>>,
    channel: Sender<bool>,
    handle: Option<JoinHandle<()>>,
    target: ConsoleTarget,

    pub quiet: bool,
}

impl Console {
    pub fn new(target: ConsoleTarget, quiet: bool) -> Self {
        let buffer = Arc::new(RwLock::new(BufWriter::new(Vec::new())));
        let buffer_clone = Arc::clone(&buffer);
        let (tx, rx) = mpsc::channel();

        // Every 100ms, flush the buffer
        let handle = spawn(move || loop {
            if quiet {
                break;
            }

            sleep(Duration::from_millis(100));

            if let Ok(mut out) = buffer_clone.write() {
                flush(&mut out, target).unwrap();

                // Has the thread been closed?
                match rx.try_recv() {
                    Ok(false) => {}
                    Ok(true) | Err(_) => {
                        break;
                    }
                }
            } else {
                break;
            }
        });

        Self {
            buffer,
            channel: tx,
            handle: Some(handle),
            target,
            quiet,
        }
    }

    pub fn is_terminal(&self) -> bool {
        match self.target {
            ConsoleTarget::Stderr => io::stderr().is_terminal(),
            ConsoleTarget::Stdout => io::stdout().is_terminal(),
        }
    }

    pub fn close(&mut self) {
        self.flush().unwrap();

        let _ = self.channel.send(true);

        self.handle.take().unwrap().join().unwrap();
    }

    pub fn flush(&self) -> miette::Result<()> {
        if let Ok(mut out) = self.buffer.write() {
            flush(&mut out, self.target).unwrap();
        }

        Ok(())
    }

    pub fn write<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        let mut buffer = self
            .buffer
            .write()
            .expect("Failed to acquire console write lock.");

        buffer.write_all(data.as_ref()).unwrap();

        // Buffer has written its data to the vec, so flush it
        if !buffer.get_ref().is_empty() {
            flush(&mut buffer, self.target).unwrap();
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
        self.close();
    }
}

fn flush(buffer: &mut BufWriter<Vec<u8>>, target: ConsoleTarget) -> io::Result<()> {
    buffer.flush()?;

    let data = buffer.get_mut().drain(0..).collect::<Vec<_>>();

    if data.is_empty() {
        return Ok(());
    }

    match target {
        ConsoleTarget::Stderr => io::stderr().lock().write_all(&data),
        ConsoleTarget::Stdout => io::stdout().lock().write_all(&data),
    }
}
