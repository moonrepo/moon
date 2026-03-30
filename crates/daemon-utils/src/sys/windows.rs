use async_stream::stream;
use futures_core::stream::Stream;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::pin::Pin;
use std::process::{Command, Stdio};
use tokio::{
    io::{self, AsyncRead, AsyncWrite},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
};
use tonic::transport::server::Connected;
use windows_sys::Win32::{Foundation, System::Threading};

pub fn is_process_alive(pid: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe {
        let handle = Threading::OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);

        if handle.is_null() {
            return false;
        }

        let mut exit_code: u32 = 0;
        let result = Threading::GetExitCodeProcess(handle, &mut exit_code);

        Foundation::CloseHandle(handle);

        result != 0 && exit_code == STILL_ACTIVE
    }
}

pub fn kill_process(pid: u32) -> std::io::Result<()> {
    const PROCESS_TERMINATE: u32 = 0x0001;

    unsafe {
        let handle = Threading::OpenProcess(PROCESS_TERMINATE, 0, pid);

        if handle.is_null() {
            let error = std::io::Error::last_os_error();

            // If the process is already gone, that's fine.
            if error.raw_os_error() != Some(87) {
                return Err(error);
            }

            return Ok(());
        }

        let terminated = Threading::TerminateProcess(handle, 1);

        Foundation::CloseHandle(handle);

        if terminated == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}

pub fn create_detached_command(exe: &Path) -> Command {
    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    const DETACH_FLAGS: u32 = 0x0000_0008 | 0x0000_0200;

    let mut command = Command::new(exe);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACH_FLAGS);

    command
}

// https://github.com/catalinsh/tonic-named-pipe-example/blob/master/src/bin/server/named_pipe_stream.rs
pub struct TonicNamedPipeServer {
    inner: NamedPipeServer,
}

impl TonicNamedPipeServer {
    pub fn new(inner: NamedPipeServer) -> Self {
        Self { inner }
    }
}

impl Connected for TonicNamedPipeServer {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}

impl AsyncRead for TonicNamedPipeServer {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TonicNamedPipeServer {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

pub fn get_named_pipe_server_stream(
    endpoint: &str,
) -> impl Stream<Item = io::Result<TonicNamedPipeServer>> {
    stream! {
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(endpoint)?;

        loop {
            server.connect().await?;

            let instance = TonicNamedPipeServer::new(server);

            yield Ok(instance);

            server = ServerOptions::new().create(endpoint)?;
        }
    }
}
