use std::io::{self, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

pub const MAX_BUFFER_SIZE: u16 = 1024;

#[derive(Clone, Copy)]
pub enum ConsoleTarget {
    Stderr,
    Stdout,
}

pub struct Console {
    buffer: Arc<RwLock<Vec<u8>>>,
    channel: Sender<bool>,
    handle: JoinHandle<()>,
    target: ConsoleTarget,
    quiet: bool,
}

impl Console {
    pub fn new(target: ConsoleTarget, quiet: bool) -> Self {
        let buffer = Arc::new(RwLock::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);
        let (tx, rx) = mpsc::channel();

        // Every 100ms, flush the buffer
        let handle = spawn(move || loop {
            if quiet {
                break;
            }

            sleep(Duration::from_millis(100));

            if let Ok(mut out) = buffer_clone.write() {
                flush(&mut out, target);

                // Has the thread been closed?
                match rx.try_recv() {
                    // If false, no
                    Ok(value) if value == false => {}
                    // If true or an error, yes
                    _ => {
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
            handle,
            target,
            quiet,
        }
    }

    pub fn close(self) {
        self.channel.send(true).unwrap();
        self.handle.join().unwrap();
    }

    pub fn write(&self, data: Vec<u8>) {
        if self.quiet {
            return;
        }

        let mut buffer = self
            .buffer
            .write()
            .expect("Failed to acquire console write lock.");

        buffer.extend(data);

        if buffer.len() > MAX_BUFFER_SIZE as usize {
            flush(&mut buffer, self.target);
        }
    }

    pub fn write_line(&self, mut data: Vec<u8>) {
        data.extend(b"\n");
        self.write(data);
    }
}

fn flush(buffer: &mut Vec<u8>, target: ConsoleTarget) {
    let data = buffer.drain(0..).collect::<Vec<_>>();

    let result = match target {
        ConsoleTarget::Stderr => io::stderr().lock().write_all(&data),
        ConsoleTarget::Stdout => io::stdout().lock().write_all(&data),
    };

    result.unwrap();
}
