#[cfg(not(windows))]
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(not(windows))]
use fs2::FileExt;
use serde::{Deserialize, Serialize};

pub const MAX_COMMAND_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload", rename_all = "kebab-case")]
pub enum InstanceCommand {
    Activate,
    Lookup(String),
    Open(Vec<String>),
}

impl InstanceCommand {
    pub fn encode(&self) -> Result<Vec<u8>, IpcError> {
        let payload = serde_json::to_vec(self)?;
        if payload.len() > MAX_COMMAND_BYTES {
            return Err(IpcError::MessageTooLarge {
                actual: payload.len(),
                maximum: MAX_COMMAND_BYTES,
            });
        }
        Ok(payload)
    }

    pub fn decode(payload: &[u8]) -> Result<Self, IpcError> {
        if payload.len() > MAX_COMMAND_BYTES {
            return Err(IpcError::MessageTooLarge {
                actual: payload.len(),
                maximum: MAX_COMMAND_BYTES,
            });
        }
        Ok(serde_json::from_slice(payload)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("instance IPC I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("instance command is invalid: {0}")]
    Invalid(#[from] serde_json::Error),
    #[error("instance command has {actual} bytes; maximum is {maximum}")]
    MessageTooLarge { actual: usize, maximum: usize },
    #[error("instance command timed out")]
    Timeout,
    #[error("single-instance support is unavailable on this platform")]
    Unsupported,
}

#[cfg(windows)]
impl From<windows::core::Error> for IpcError {
    fn from(error: windows::core::Error) -> Self {
        IpcError::Io(io::Error::from_raw_os_error(error.code().0))
    }
}

#[derive(Debug)]
pub enum InstanceLock {
    Primary(PrimaryInstanceLock),
    Secondary,
}

#[derive(Debug)]
pub struct PrimaryInstanceLock {
    #[cfg(not(windows))]
    file: File,
    #[cfg(windows)]
    handle: windows::Win32::Foundation::HANDLE,
}

impl InstanceLock {
    pub fn acquire(runtime_root: &Path, name: &str) -> Result<Self, IpcError> {
        acquire_lock(runtime_root, name)
    }
}

#[cfg(not(windows))]
fn acquire_lock(runtime_root: &Path, name: &str) -> Result<InstanceLock, IpcError> {
    fs::create_dir_all(runtime_root)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(runtime_root.join(format!("{name}.lock")))?;
    match file.try_lock_exclusive() {
        Ok(()) => Ok(InstanceLock::Primary(PrimaryInstanceLock { file })),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(InstanceLock::Secondary),
        Err(error) => Err(error.into()),
    }
}

#[cfg(not(windows))]
impl Drop for PrimaryInstanceLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

#[cfg(windows)]
fn acquire_lock(_runtime_root: &Path, name: &str) -> Result<InstanceLock, IpcError> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let user_scoped_name = format!("Local\\LexiForge-{name}");
    let wide: Vec<u16> = std::ffi::OsStr::new(&user_scoped_name)
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: The pointer is valid and NUL-terminated for the duration of the call.
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr()))? };
    // SAFETY: GetLastError has no preconditions and is read immediately after CreateMutexW.
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        // SAFETY: CreateMutexW returned an owned valid handle.
        unsafe { CloseHandle(handle)? };
        Ok(InstanceLock::Secondary)
    } else {
        Ok(InstanceLock::Primary(PrimaryInstanceLock { handle }))
    }
}

#[cfg(windows)]
impl Drop for PrimaryInstanceLock {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        // SAFETY: This object exclusively owns the handle returned by CreateMutexW.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

#[derive(Debug, Clone)]
pub struct LocalEndpoint {
    root: PathBuf,
    name: String,
}

impl LocalEndpoint {
    pub fn new(runtime_root: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            root: runtime_root.into(),
            name: name.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bind(&self) -> Result<LocalServer, IpcError> {
        bind_endpoint(self)
    }

    pub fn send(&self, command: &InstanceCommand, timeout: Duration) -> Result<(), IpcError> {
        send_command(self, command, timeout)
    }
}

#[cfg(unix)]
#[derive(Debug)]
pub struct LocalServer {
    listener: std::os::unix::net::UnixListener,
    path: PathBuf,
}

#[cfg(unix)]
fn socket_path(endpoint: &LocalEndpoint) -> PathBuf {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    endpoint.name.hash(&mut hasher);
    endpoint.root.join(format!("{:016x}.sock", hasher.finish()))
}

#[cfg(unix)]
fn bind_endpoint(endpoint: &LocalEndpoint) -> Result<LocalServer, IpcError> {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(&endpoint.root)?;
    fs::set_permissions(&endpoint.root, fs::Permissions::from_mode(0o700))?;
    let path = socket_path(endpoint);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    let listener = std::os::unix::net::UnixListener::bind(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok(LocalServer { listener, path })
}

#[cfg(unix)]
fn send_command(
    endpoint: &LocalEndpoint,
    command: &InstanceCommand,
    timeout: Duration,
) -> Result<(), IpcError> {
    use std::io::Write;

    let stream = std::os::unix::net::UnixStream::connect(socket_path(endpoint))?;
    stream.set_write_timeout(Some(timeout))?;
    let payload = command.encode()?;
    (&stream).write_all(&(payload.len() as u32).to_le_bytes())?;
    (&stream).write_all(&payload)?;
    Ok(())
}

#[cfg(unix)]
impl LocalServer {
    pub fn recv(&self, timeout: Duration) -> Result<InstanceCommand, IpcError> {
        use std::io::Read;

        self.listener.set_nonblocking(true)?;
        let started = std::time::Instant::now();
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(timeout))?;
                    let mut length = [0_u8; 4];
                    stream.read_exact(&mut length)?;
                    let length = u32::from_le_bytes(length) as usize;
                    if length > MAX_COMMAND_BYTES {
                        return Err(IpcError::MessageTooLarge {
                            actual: length,
                            maximum: MAX_COMMAND_BYTES,
                        });
                    }
                    let mut payload = vec![0_u8; length];
                    stream.read_exact(&mut payload)?;
                    return InstanceCommand::decode(&payload);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    if started.elapsed() >= timeout {
                        return Err(IpcError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

#[cfg(unix)]
impl Drop for LocalServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(all(not(unix), not(windows)))]
#[derive(Debug)]
pub struct LocalServer;

#[cfg(all(not(unix), not(windows)))]
fn bind_endpoint(_: &LocalEndpoint) -> Result<LocalServer, IpcError> {
    Err(IpcError::Unsupported)
}

#[cfg(all(not(unix), not(windows)))]
fn send_command(_: &LocalEndpoint, _: &InstanceCommand, _: Duration) -> Result<(), IpcError> {
    Err(IpcError::Unsupported)
}

#[cfg(all(not(unix), not(windows)))]
impl LocalServer {
    pub fn recv(&self, _: Duration) -> Result<InstanceCommand, IpcError> {
        Err(IpcError::Unsupported)
    }
}

#[cfg(windows)]
mod windows_pipe;

#[cfg(windows)]
pub use windows_pipe::LocalServer;
#[cfg(windows)]
use windows_pipe::{bind_endpoint, send_command};

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_name(label: &str) -> String {
        format!(
            "lexiforge-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn command_round_trip_preserves_lookup_and_open_requests() {
        for command in [
            InstanceCommand::Activate,
            InstanceCommand::Lookup("xin chào".into()),
            InstanceCommand::Open(vec!["one.ld2".into(), "two.tsv".into()]),
        ] {
            assert_eq!(
                InstanceCommand::decode(&command.encode().unwrap()).unwrap(),
                command
            );
        }
    }

    #[test]
    fn oversized_command_is_rejected_before_decode() {
        let payload = vec![b'x'; MAX_COMMAND_BYTES + 1];
        assert!(matches!(
            InstanceCommand::decode(&payload),
            Err(IpcError::MessageTooLarge { .. })
        ));
    }

    #[test]
    fn only_one_primary_instance_can_hold_the_lock() {
        let root = tempfile::tempdir().unwrap();
        let name = unique_name("lock");
        let first = InstanceLock::acquire(root.path(), &name).unwrap();
        let second = InstanceLock::acquire(root.path(), &name).unwrap();
        assert!(matches!(first, InstanceLock::Primary(_)));
        assert!(matches!(second, InstanceLock::Secondary));
    }

    #[cfg(unix)]
    #[test]
    fn local_endpoint_forwards_a_command_to_the_primary() {
        let root = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::new(root.path(), unique_name("endpoint"));
        let server = endpoint.bind().unwrap();
        let client_endpoint = endpoint.clone();
        let thread = std::thread::spawn(move || {
            client_endpoint
                .send(
                    &InstanceCommand::Lookup("rust".into()),
                    Duration::from_secs(1),
                )
                .unwrap();
        });

        let command = server.recv(Duration::from_secs(1)).unwrap();

        thread.join().unwrap();
        assert_eq!(command, InstanceCommand::Lookup("rust".into()));
    }
}
