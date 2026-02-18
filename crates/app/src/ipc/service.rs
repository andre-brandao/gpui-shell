use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use tokio::net::UnixListener as TokioUnixListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::args::Args;

use super::messages::{IpcMessage, command_for_secondary, decode_command, encode_command};

pub type IpcReceiver = mpsc::UnboundedReceiver<IpcMessage>;

enum AcquireResult {
    Primary(IpcSubscriber),
    Secondary,
    Error(String),
}

/// Subscriber for IPC messages from other instances.
pub struct IpcSubscriber {
    /// The bound Unix listener (not yet accepting connections)
    listener: Option<TokioUnixListener>,
    /// Socket path for cleanup
    socket_path: PathBuf,
}

impl std::fmt::Debug for IpcSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcSubscriber")
            .field("socket_path", &self.socket_path)
            .finish()
    }
}

impl IpcSubscriber {
    /// Initialize IPC single-instance handling.
    ///
    /// Returns `Some(IpcSubscriber)` when this process should continue as
    /// the primary instance, otherwise `None`.
    ///
    /// This performs one retry without initial input when the first acquire
    /// attempt fails with an error.
    pub fn init(args: &Args) -> Option<IpcSubscriber> {
        match Self::acquire(args) {
            AcquireResult::Primary(subscriber) => Some(subscriber),
            AcquireResult::Secondary => None,
            AcquireResult::Error(err) => {
                error!("IPC service error: {}", err);
                warn!("Retrying IPC acquire without initial input");

                let retry_args = Args { input: None };
                match Self::acquire(&retry_args) {
                    AcquireResult::Primary(subscriber) => Some(subscriber),
                    AcquireResult::Secondary => None,
                    AcquireResult::Error(retry_err) => {
                        error!("Failed to acquire IPC service on retry: {}", retry_err);
                        None
                    }
                }
            }
        }
    }

    /// Try to become the primary instance or signal an existing one.
    ///
    /// This function uses synchronous I/O for the secondary path to minimize
    /// startup latency - no async runtime needed to signal an existing instance.
    ///
    /// When becoming the primary instance, a tokio runtime must be active.
    ///
    /// If no other instance is running, prepares a Unix socket listener
    /// and returns `AcquireResult::Primary` with the subscriber.
    ///
    /// If another instance is already running, sends a command
    /// to it synchronously and returns `AcquireResult::Secondary`.
    fn acquire(args: &Args) -> AcquireResult {
        let path = socket_path();
        let command = command_for_secondary(args);

        // Try to connect to existing instance (fast, synchronous path)
        if let Ok(mut stream) = UnixStream::connect(&path) {
            // Set a short timeout for the write
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));

            let payload = encode_command(&command);
            if let Err(e) = stream.write_all(payload.as_bytes()) {
                error!("Failed to send message to existing instance: {}", e);
                return AcquireResult::Error(format!(
                    "Failed to signal existing instance: {}",
                    e
                ));
            }

            // Flush and shutdown to signal end of message
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);

            info!("Successfully signaled existing instance");
            return AcquireResult::Secondary;
        }

        // No existing instance, become the primary
        // Remove stale socket if it exists
        if path.exists()
            && let Err(e) = std::fs::remove_file(&path)
        {
            warn!("Failed to remove stale socket: {}", e);
        }

        // Create the Unix listener
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind Unix socket: {}", e);
                return AcquireResult::Error(format!("Failed to create socket: {}", e));
            }
        };

        // Set non-blocking for tokio compatibility
        if let Err(e) = listener.set_nonblocking(true) {
            error!("Failed to set socket non-blocking: {}", e);
            return AcquireResult::Error(format!("Failed to configure socket: {}", e));
        }

        // Convert to tokio listener (doesn't require runtime yet)
        let tokio_listener = match TokioUnixListener::from_std(listener) {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to create tokio listener: {}", e);
                return AcquireResult::Error(format!("Failed to create async listener: {}", e));
            }
        };

        info!("Prepared as primary instance, socket at {:?}", path);
        let subscriber = IpcSubscriber {
            listener: Some(tokio_listener),
            socket_path: path,
        };

        AcquireResult::Primary(subscriber)
    }

    /// Start the listener and return a receiver for IPC messages.
    ///
    /// This must be called from within a tokio runtime context.
    /// Returns a receiver that yields `IpcMessage` items.
    pub fn start_listener(&mut self) -> IpcReceiver {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Take the listener and spawn the accept loop
        if let Some(listener) = self.listener.take() {
            let path_clone = self.socket_path.clone();
            tokio::spawn(async move {
                accept_loop(listener, sender, path_clone).await;
            });
        }

        receiver
    }
}

/// Accept loop for incoming connections.
async fn accept_loop(
    listener: TokioUnixListener,
    sender: mpsc::UnboundedSender<IpcMessage>,
    socket_path: PathBuf,
) {
    use std::sync::atomic::AtomicU64;
    static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    info!("Socket listener started at {:?}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let sender = sender.clone();
                tokio::spawn(async move {
                    handle_connection(stream, sender, &REQUEST_COUNTER).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                // Small delay to prevent tight loop on persistent errors
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

/// Handle a single connection from a secondary instance.
async fn handle_connection(
    stream: tokio::net::UnixStream,
    sender: mpsc::UnboundedSender<IpcMessage>,
    counter: &std::sync::atomic::AtomicU64,
) {
    use tokio::io::AsyncReadExt;

    let mut stream = stream;
    let mut buffer = Vec::with_capacity(256);

    // Read with timeout
    let read_result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        stream.read_to_end(&mut buffer),
    )
    .await;

    let payload = match read_result {
        Ok(Ok(_)) => String::from_utf8_lossy(&buffer).to_string(),
        Ok(Err(e)) => {
            debug!("Error reading from socket: {}", e);
            String::new()
        }
        Err(_) => {
            debug!("Timeout reading from socket");
            String::new()
        }
    };

    let request_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let command = decode_command(&payload);
    let message = IpcMessage {
        id: request_id,
        command,
    };

    if let Err(e) = sender.send(message) {
        error!("Failed to send request to channel: {}", e);
    }
}

impl Drop for IpcSubscriber {
    fn drop(&mut self) {
        // Clean up socket file on shutdown
        let path = socket_path();
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!("Failed to remove socket on shutdown: {}", e);
            } else {
                debug!("Removed socket file on shutdown");
            }
        }
    }
}

/// Get the socket path for IPC.
fn socket_path() -> PathBuf {
    // Prefer XDG_RUNTIME_DIR for security (user-only access, tmpfs)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("gpuishell.sock")
    } else {
        // Fallback to /tmp with UID for uniqueness across users
        let uid = std::env::var("UID")
            .or_else(|_| std::env::var("SUDO_UID"))
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string());
        PathBuf::from(format!("/tmp/gpuishell-{}.sock", uid))
    }
}
