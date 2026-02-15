//! Shell service for single-instance IPC via Unix sockets.
//!
//! This module provides a service that ensures only one instance of gpuishell
//! runs at a time. When a second instance is launched, it sends a message to
//! the running instance via a Unix socket and exits immediately.
//!
//! Uses Unix sockets instead of D-Bus for near-instant communication.

use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use tokio::net::UnixListener as TokioUnixListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Get the socket path for IPC.
fn socket_path() -> PathBuf {
    // Prefer XDG_RUNTIME_DIR for security (user-only access, tmpfs)
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("gpuishell.sock")
    } else {
        // Fallback to /tmp with UID for uniqueness across users
        let uid = nix::unistd::getuid();
        PathBuf::from(format!("/tmp/gpuishell-{}.sock", uid))
    }
}

/// Command to open the launcher with optional prefilled input.
#[derive(Debug, Clone, Default)]
pub struct LauncherRequest {
    /// Optional input to prefill in the launcher.
    pub input: Option<String>,
    /// Unique request ID to ensure each request is distinct.
    pub id: u64,
}

/// Shell service data - tracks pending launcher requests.
#[derive(Debug, Clone, Default)]
pub struct ShellData {
    /// Pending launcher request, if any.
    pub launcher_request: Option<LauncherRequest>,
}

/// Result of trying to acquire the single-instance lock.
pub enum InstanceResult {
    /// This is the primary instance - the service is now running.
    Primary(ShellSubscriber),
    /// Another instance is already running - a message was sent to it.
    Secondary,
    /// Failed to set up IPC.
    Error(String),
}

/// Receiver for launcher requests from other instances.
pub struct ShellSubscriber {
    /// The bound Unix listener (not yet accepting connections)
    listener: Option<TokioUnixListener>,
    /// Initial input to queue when listener starts
    initial_input: Option<String>,
    /// Socket path for cleanup
    socket_path: PathBuf,
}

impl std::fmt::Debug for ShellSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSubscriber")
            .field("socket_path", &self.socket_path)
            .finish()
    }
}

impl ShellSubscriber {
    /// Try to become the primary instance or signal an existing one.
    ///
    /// This function uses synchronous I/O for the secondary path to minimize
    /// startup latency - no async runtime needed to signal an existing instance.
    ///
    /// If no other instance is running, prepares a Unix socket listener
    /// and returns `InstanceResult::Primary` with the subscriber.
    /// Call `start_listener()` to begin accepting connections.
    ///
    /// If another instance is already running, sends a launcher open request
    /// to it synchronously and returns `InstanceResult::Secondary`.
    pub fn acquire(input: Option<String>) -> InstanceResult {
        let path = socket_path();

        // Try to connect to existing instance (fast, synchronous path)
        if let Ok(mut stream) = UnixStream::connect(&path) {
            // Set a short timeout for the write
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(100)));

            // Send the input (empty string means no input)
            let message = input.as_deref().unwrap_or("");
            if let Err(e) = stream.write_all(message.as_bytes()) {
                error!("Failed to send message to existing instance: {}", e);
                return InstanceResult::Error(format!("Failed to signal existing instance: {}", e));
            }

            // Flush and shutdown to signal end of message
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);

            info!("Successfully signaled existing instance to open launcher");
            return InstanceResult::Secondary;
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
                return InstanceResult::Error(format!("Failed to create socket: {}", e));
            }
        };

        // Set non-blocking for tokio compatibility
        if let Err(e) = listener.set_nonblocking(true) {
            error!("Failed to set socket non-blocking: {}", e);
            return InstanceResult::Error(format!("Failed to configure socket: {}", e));
        }

        // Convert to tokio listener (doesn't require runtime yet)
        let tokio_listener = match TokioUnixListener::from_std(listener) {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to create tokio listener: {}", e);
                return InstanceResult::Error(format!("Failed to create async listener: {}", e));
            }
        };

        info!("Prepared as primary instance, socket at {:?}", path);
        InstanceResult::Primary(ShellSubscriber {
            listener: Some(tokio_listener),
            initial_input: input,
            socket_path: path,
        })
    }

    /// Start the listener and return a receiver for launcher requests.
    ///
    /// This must be called from within a tokio runtime context.
    /// Returns a receiver that yields `LauncherRequest` items.
    pub fn start_listener(&mut self) -> mpsc::UnboundedReceiver<LauncherRequest> {
        let (sender, receiver) = mpsc::unbounded_channel();

        // If we have an initial input, send it immediately
        if let Some(input_text) = self.initial_input.take() {
            let _ = sender.send(LauncherRequest {
                input: Some(input_text),
                id: 0,
            });
        }

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
    sender: mpsc::UnboundedSender<LauncherRequest>,
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
    sender: mpsc::UnboundedSender<LauncherRequest>,
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

    let input = match read_result {
        Ok(Ok(_)) => {
            let s = String::from_utf8_lossy(&buffer).to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        Ok(Err(e)) => {
            debug!("Error reading from socket: {}", e);
            None
        }
        Err(_) => {
            debug!("Timeout reading from socket");
            None
        }
    };

    let request_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    info!(
        "Received launcher request: id={}, input={:?}",
        request_id, input
    );

    let request = LauncherRequest {
        input,
        id: request_id,
    };

    if let Err(e) = sender.send(request) {
        error!("Failed to send request to channel: {}", e);
    }
}

impl Drop for ShellSubscriber {
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
