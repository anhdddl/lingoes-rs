use std::io;
use std::os::windows::ffi::OsStrExt;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    ReadFile, WriteFile,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_TYPE_MESSAGE, PIPE_WAIT, WaitNamedPipeW,
};
use windows::core::PCWSTR;

use super::{InstanceCommand, IpcError, LocalEndpoint, MAX_COMMAND_BYTES};

#[derive(Debug)]
pub struct LocalServer {
    handle: windows::Win32::Foundation::HANDLE,
}

fn pipe_name(endpoint: &LocalEndpoint) -> Vec<u16> {
    let name = format!(r"\\.\pipe\LexiForge-{}", endpoint.name);
    std::ffi::OsStr::new(&name)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

pub(super) fn bind_endpoint(endpoint: &LocalEndpoint) -> Result<LocalServer, IpcError> {
    let name = pipe_name(endpoint);
    // SAFETY: name is NUL-terminated. The default DACL is inherited from the per-user process
    // token, and the pipe is local-only because PIPE_REJECT_REMOTE_CLIENTS is set.
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_MESSAGE
                | PIPE_READMODE_MESSAGE
                | PIPE_WAIT
                | windows::Win32::System::Pipes::PIPE_REJECT_REMOTE_CLIENTS,
            1,
            MAX_COMMAND_BYTES as u32,
            MAX_COMMAND_BYTES as u32,
            0,
            None,
        )
    };
    if handle.is_invalid() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }
    Ok(LocalServer { handle })
}

pub(super) fn send_command(
    endpoint: &LocalEndpoint,
    command: &InstanceCommand,
    timeout: Duration,
) -> Result<(), IpcError> {
    let name = pipe_name(endpoint);
    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
    // SAFETY: name is NUL-terminated and remains alive for the call.
    let waited = unsafe { WaitNamedPipeW(PCWSTR(name.as_ptr()), timeout_ms) };
    if !waited.as_bool() {
        return Err(IpcError::Timeout);
    }
    // SAFETY: Arguments describe a local named pipe opened synchronously.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            (GENERIC_READ | GENERIC_WRITE).0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?
    };
    let payload = command.encode()?;
    let length = (payload.len() as u32).to_le_bytes();
    write_all(handle, &length)?;
    write_all(handle, &payload)?;
    // SAFETY: handle is owned by this function and no longer used afterward.
    unsafe { CloseHandle(handle)? };
    Ok(())
}

fn write_all(handle: windows::Win32::Foundation::HANDLE, bytes: &[u8]) -> Result<(), IpcError> {
    let mut written = 0;
    // SAFETY: bytes points to initialized memory valid for the duration of this synchronous call.
    unsafe {
        WriteFile(handle, Some(bytes), Some(&mut written), None)?;
    }
    if written as usize != bytes.len() {
        return Err(io::Error::new(io::ErrorKind::WriteZero, "short named-pipe write").into());
    }
    Ok(())
}

impl LocalServer {
    pub fn recv(&self, timeout: Duration) -> Result<InstanceCommand, IpcError> {
        let started = Instant::now();
        loop {
            // SAFETY: handle owns a named-pipe server instance. Null OVERLAPPED requests a
            // synchronous connection.
            match unsafe { ConnectNamedPipe(self.handle, None) } {
                Ok(()) => break,
                Err(error) if error.code().0 as u32 == ERROR_PIPE_BUSY.0 => {
                    if started.elapsed() >= timeout {
                        return Err(IpcError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(io::Error::from_raw_os_error(error.code().0).into()),
            }
        }
        let mut length = [0_u8; 4];
        read_exact(self.handle, &mut length)?;
        let length = u32::from_le_bytes(length) as usize;
        if length > MAX_COMMAND_BYTES {
            return Err(IpcError::MessageTooLarge {
                actual: length,
                maximum: MAX_COMMAND_BYTES,
            });
        }
        let mut payload = vec![0_u8; length];
        read_exact(self.handle, &mut payload)?;
        // SAFETY: self.handle is a connected server pipe.
        unsafe { DisconnectNamedPipe(self.handle)? };
        InstanceCommand::decode(&payload)
    }
}

fn read_exact(
    handle: windows::Win32::Foundation::HANDLE,
    mut bytes: &mut [u8],
) -> Result<(), IpcError> {
    while !bytes.is_empty() {
        let mut read = 0;
        // SAFETY: bytes points to writable memory valid for this synchronous call.
        unsafe { ReadFile(handle, Some(bytes), Some(&mut read), None)? };
        if read == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "named pipe closed").into());
        }
        bytes = &mut bytes[read as usize..];
    }
    Ok(())
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        // SAFETY: this object exclusively owns the server pipe handle.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}
