use crate::{ChildExit, Command, Output, ProcessError};
use bytes::Bytes;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Child, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

/// Configuration for capturing a command with the blocking executor.
#[derive(Clone, Debug, Default)]
pub struct CaptureOptions {
    /// Kill the command if it runs longer than this duration.
    pub timeout: Option<Duration>,

    /// Maximum combined number of bytes captured from stdout and stderr.
    pub output_limit: Option<usize>,
}

#[derive(Debug)]
pub struct FileOutput {
    pub exit: ChildExit,
    pub stderr_len: u64,
    pub stdout_len: u64,
}

impl Command {
    /// Execute the command synchronously and capture byte-safe output.
    ///
    /// Any descendants still running after capture completes are terminated. Set a timeout when a
    /// descendant may keep an output pipe open, as capture otherwise waits for all output readers.
    pub fn exec_capture_output_blocking(
        &mut self,
        options: &CaptureOptions,
    ) -> miette::Result<Output> {
        let output =
            BlockingCapture::spawn(self, options, CaptureTarget::Memory, CaptureTarget::Memory)?
                .wait()?;
        let output = Output {
            exit: output.exit,
            stderr: Bytes::from(output.stderr.into_memory()),
            stdout: Bytes::from(output.stdout.into_memory()),
        };

        self.handle_nonzero_status(&output, true)?;

        Ok(output)
    }

    /// Execute the command synchronously and capture output in caller-owned files.
    ///
    /// Any descendants still running after capture completes are terminated. Set a timeout when a
    /// descendant may keep an output pipe open, as capture otherwise waits for all output readers.
    pub fn exec_capture_output_to_files_blocking(
        &mut self,
        options: &CaptureOptions,
        stdout: File,
        stderr: File,
    ) -> miette::Result<FileOutput> {
        let output = BlockingCapture::spawn(
            self,
            options,
            CaptureTarget::File(stdout),
            CaptureTarget::File(stderr),
        )?
        .wait()?;
        let output = FileOutput {
            exit: output.exit,
            stderr_len: output.stderr.into_file_len(),
            stdout_len: output.stdout.into_file_len(),
        };
        self.handle_nonzero_status(
            &Output {
                exit: output.exit.clone(),
                stderr: Bytes::new(),
                stdout: Bytes::new(),
            },
            false,
        )?;

        Ok(output)
    }
}

enum CaptureTarget {
    File(File),
    Memory,
}

enum CapturedOutput {
    File(u64),
    Memory(Vec<u8>),
}

impl CapturedOutput {
    fn into_file_len(self) -> u64 {
        match self {
            Self::File(len) => len,
            Self::Memory(_) => unreachable!("output was not captured to a file"),
        }
    }

    fn into_memory(self) -> Vec<u8> {
        match self {
            Self::File(_) => unreachable!("output was not captured to memory"),
            Self::Memory(output) => output,
        }
    }
}

struct CaptureOutput {
    exit: ChildExit,
    stderr: CapturedOutput,
    stdout: CapturedOutput,
}

type ReaderHandle = thread::JoinHandle<io::Result<CapturedOutput>>;
type WriterHandle = thread::JoinHandle<io::Result<()>>;

struct BlockingCapture {
    bin: String,
    child: ChildGuard,
    deadline: Option<Instant>,
    exceeded: Arc<AtomicBool>,
    output_limit: Option<usize>,
    stderr_reader: Option<ReaderHandle>,
    stdin_writer: Option<WriterHandle>,
    stdout_reader: Option<ReaderHandle>,
    timeout: Option<Duration>,
}

impl BlockingCapture {
    fn spawn(
        command: &Command,
        options: &CaptureOptions,
        stdout_target: CaptureTarget,
        stderr_target: CaptureTarget,
    ) -> miette::Result<Self> {
        let bin = command.get_bin_name();
        let deadline = options
            .timeout
            .map(|timeout| {
                Instant::now()
                    .checked_add(timeout)
                    .ok_or_else(|| capture_error(&bin, "process timeout is too large"))
            })
            .transpose()?;
        let mut process = command.create_sync_command()?;

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            process.process_group(0);
        }

        if command.should_pass_stdin() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::null());
        }

        process.stdout(Stdio::piped()).stderr(Stdio::piped());

        let child = process.spawn().map_err(|error| ProcessError::Capture {
            bin: bin.clone(),
            error: Box::new(error),
        })?;
        let mut child = ChildGuard::new(child).map_err(|error| ProcessError::Capture {
            bin: bin.clone(),
            error: Box::new(error),
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| capture_error(&bin, "process stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| capture_error(&bin, "process stderr was not piped"))?;
        let exceeded = Arc::new(AtomicBool::new(false));
        let output_size = Arc::new(AtomicUsize::new(0));
        let stdout_reader = read_bounded_output(
            stdout,
            stdout_target,
            options.output_limit,
            Arc::clone(&output_size),
            Arc::clone(&exceeded),
        );
        let stderr_reader = read_bounded_output(
            stderr,
            stderr_target,
            options.output_limit,
            Arc::clone(&output_size),
            Arc::clone(&exceeded),
        );
        let stdin_writer = child.stdin.take().map(|mut stdin| {
            let input = command.input.join(std::ffi::OsStr::new(" "));
            let input = input.as_encoded_bytes().to_vec();

            thread::spawn(move || match stdin.write_all(&input) {
                Err(error) if error.kind() != io::ErrorKind::BrokenPipe => Err(error),
                _ => Ok(()),
            })
        });

        Ok(Self {
            bin,
            child,
            deadline,
            exceeded,
            output_limit: options.output_limit,
            stderr_reader: Some(stderr_reader),
            stdin_writer,
            stdout_reader: Some(stdout_reader),
            timeout: options.timeout,
        })
    }

    fn wait(mut self) -> miette::Result<CaptureOutput> {
        let mut status = None;

        loop {
            if self.exceeded.load(Ordering::Acquire) {
                return self.abort(ProcessError::OutputLimitExceeded {
                    bin: self.bin.clone(),
                    limit: self.output_limit.unwrap_or_default(),
                });
            }

            if status.is_none() {
                match self.child.try_wait() {
                    Ok(exit_status) => status = exit_status,
                    Err(error) => {
                        return self.abort(ProcessError::Capture {
                            bin: self.bin.clone(),
                            error: Box::new(error),
                        });
                    }
                }
            }

            let stdin_finished = self
                .stdin_writer
                .as_ref()
                .is_none_or(thread::JoinHandle::is_finished);

            if status.is_some()
                && stdin_finished
                && self
                    .stdout_reader
                    .as_ref()
                    .is_some_and(|reader| reader.is_finished())
                && self
                    .stderr_reader
                    .as_ref()
                    .is_some_and(|reader| reader.is_finished())
            {
                break;
            }

            if self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return self.abort(ProcessError::Timeout {
                    bin: self.bin.clone(),
                    timeout: self.timeout.unwrap_or_default(),
                });
            }

            thread::sleep(Duration::from_millis(10));
        }

        let stdout = self.join_stdout()?;
        let stderr = self.join_stderr()?;
        self.join_stdin()?;

        if self.exceeded.load(Ordering::Acquire) {
            return self.abort(ProcessError::OutputLimitExceeded {
                bin: self.bin.clone(),
                limit: self.output_limit.unwrap_or_default(),
            });
        }

        let output = CaptureOutput {
            exit: crate::shared_child::convert_exit_status(
                status.expect("completed process has an exit status"),
                None,
            ),
            stdout,
            stderr,
        };

        self.child.finish();

        Ok(output)
    }

    fn abort<T>(&mut self, error: ProcessError) -> miette::Result<T> {
        self.child.terminate();
        self.discard_workers();

        Err(error.into())
    }

    fn discard_workers(&mut self) {
        if let Some(reader) = self.stdout_reader.take() {
            discard_reader(reader);
        }

        if let Some(reader) = self.stderr_reader.take() {
            discard_reader(reader);
        }

        if let Some(writer) = self.stdin_writer.take() {
            discard_writer(writer);
        }
    }

    fn join_stdout(&mut self) -> miette::Result<CapturedOutput> {
        let reader = self
            .stdout_reader
            .take()
            .expect("stdout reader has not been joined");

        match join_reader(&self.bin, reader) {
            Ok(output) => Ok(output),
            Err(error) => {
                self.child.terminate();
                self.discard_workers();
                Err(error)
            }
        }
    }

    fn join_stderr(&mut self) -> miette::Result<CapturedOutput> {
        let reader = self
            .stderr_reader
            .take()
            .expect("stderr reader has not been joined");

        match join_reader(&self.bin, reader) {
            Ok(output) => Ok(output),
            Err(error) => {
                self.child.terminate();
                self.discard_workers();
                Err(error)
            }
        }
    }

    fn join_stdin(&mut self) -> miette::Result<()> {
        let Some(writer) = self.stdin_writer.take() else {
            return Ok(());
        };

        match join_writer(&self.bin, writer) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.child.terminate();
                self.discard_workers();
                Err(error)
            }
        }
    }
}

struct ChildGuard {
    child: Child,
    armed: bool,
    #[cfg(windows)]
    job: JobObject,
}

impl ChildGuard {
    fn new(child: Child) -> io::Result<Self> {
        #[cfg(windows)]
        let job = match JobObject::assign(&child) {
            Ok(job) => job,
            Err(error) => {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();

                return Err(error);
            }
        };

        Ok(Self {
            child,
            armed: true,
            #[cfg(windows)]
            job,
        })
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn finish(&mut self) {
        #[cfg(unix)]
        terminate_process_group(&self.child);

        self.disarm();
    }

    fn terminate(&mut self) {
        if self.armed {
            #[cfg(unix)]
            terminate_process_group(&self.child);

            #[cfg(windows)]
            self.job.terminate();

            let _ = self.child.kill();
            let _ = self.child.wait();
            self.disarm();
        }
    }
}

impl std::ops::Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn capture_error(bin: &str, message: &str) -> ProcessError {
    ProcessError::Capture {
        bin: bin.to_owned(),
        error: Box::new(io::Error::other(message)),
    }
}

fn read_bounded_output<R: Read + Send + 'static>(
    mut reader: R,
    target: CaptureTarget,
    limit: Option<usize>,
    output_size: Arc<AtomicUsize>,
    exceeded: Arc<AtomicBool>,
) -> ReaderHandle {
    thread::spawn(move || {
        let mut output = match target {
            CaptureTarget::File(file) => OutputWriter::File(file),
            CaptureTarget::Memory => OutputWriter::Memory(Vec::new()),
        };
        let mut buffer = [0; 16 * 1024];
        let mut output_len = 0u64;

        loop {
            let read = reader.read(&mut buffer)?;

            if read == 0 {
                break;
            }

            if let Some(limit) = limit {
                let previous_size = output_size
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |size| {
                        Some(size.saturating_add(read))
                    })
                    .unwrap_or(usize::MAX);

                if previous_size.saturating_add(read) > limit {
                    exceeded.store(true, Ordering::Release);
                    continue;
                }
            }

            output.write_all(&buffer[..read])?;
            output_len = output_len.saturating_add(read as u64);
        }

        Ok(match output {
            OutputWriter::File(_) => CapturedOutput::File(output_len),
            OutputWriter::Memory(output) => CapturedOutput::Memory(output),
        })
    })
}

enum OutputWriter {
    File(File),
    Memory(Vec<u8>),
}

impl Write for OutputWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::File(file) => file.write(buffer),
            Self::Memory(output) => output.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(file) => file.flush(),
            Self::Memory(output) => output.flush(),
        }
    }
}

#[cfg(unix)]
fn terminate_process_group(child: &Child) {
    if let Ok(pid) = i32::try_from(child.id()) {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
struct JobObject(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl JobObject {
    fn assign(child: &Child) -> io::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Foundation::HANDLE,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
        };

        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };

        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }

        let job = Self(handle);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };

        if configured == 0 {
            return Err(io::Error::last_os_error());
        }

        let process = child.as_raw_handle() as HANDLE;
        let assigned = unsafe { AssignProcessToJobObject(job.0, process) };

        if assigned == 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(job)
    }

    fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

fn discard_reader(reader: ReaderHandle) {
    for _ in 0..10 {
        if reader.is_finished() {
            let _ = reader.join();
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn discard_writer(writer: thread::JoinHandle<io::Result<()>>) {
    for _ in 0..10 {
        if writer.is_finished() {
            let _ = writer.join();
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn join_reader(bin: &str, reader: ReaderHandle) -> miette::Result<CapturedOutput> {
    reader
        .join()
        .map_err(|_| capture_error(bin, "process output reader panicked"))?
        .map_err(|error| ProcessError::Capture {
            bin: bin.to_owned(),
            error: Box::new(error),
        })
        .map_err(Into::into)
}

fn join_writer(bin: &str, writer: thread::JoinHandle<io::Result<()>>) -> miette::Result<()> {
    writer
        .join()
        .map_err(|_| capture_error(bin, "process input writer panicked"))?
        .map_err(|error| ProcessError::WriteInput {
            bin: bin.to_owned(),
            error: Box::new(error),
        })
        .map_err(Into::into)
}
