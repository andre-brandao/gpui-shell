//! Shell service for single-instance IPC via D-Bus.
//!
//! This module provides a service that ensures only one instance of gpuishell
//! runs at a time. When a second instance is launched, it sends a message to
//! the running instance via D-Bus and exits.
//!
//! The service exposes a signal-based interface for reacting to launcher open
//! requests from other processes.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_signals::signal::{Mutable, MutableSignalCloned};
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use zbus::{Connection, connection, interface};

/// D-Bus well-known name for the shell service.
const DBUS_NAME: &str = "org.gpuishell.Shell";

/// D-Bus object path for the shell interface.
const DBUS_PATH: &str = "/org/gpuishell/Shell";

/// Global counter to ensure each request is unique.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Command to open the launcher with optional prefilled input.
#[derive(Debug, Clone, Default)]
pub struct LauncherRequest {
    /// Optional input to prefill in the launcher.
    pub input: Option<String>,
    /// Unique request ID to ensure signal fires for each request.
    pub id: u64,
}

/// Shell service data - tracks pending launcher requests.
#[derive(Debug, Clone, Default)]
pub struct ShellData {
    /// Pending launcher request, if any.
    pub launcher_request: Option<LauncherRequest>,
}

/// D-Bus interface for receiving commands from other instances.
struct ShellInterface {
    data: Mutable<ShellData>,
}

#[interface(name = "org.gpuishell.Shell")]
impl ShellInterface {
    /// Open the launcher, optionally with prefilled input.
    async fn open_launcher(&self, input: &str) {
        let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        info!(
            "Received OpenLauncher request via D-Bus: input={:?}, id={}",
            input, request_id
        );

        let request = LauncherRequest {
            input: if input.is_empty() {
                None
            } else {
                Some(input.to_string())
            },
            id: request_id,
        };

        info!("Setting ShellData with launcher_request id={}", request_id);
        self.data.set(ShellData {
            launcher_request: Some(request),
        });
    }
}

/// Event-driven shell subscriber for single-instance IPC.
///
/// This subscriber manages the D-Bus service for receiving commands
/// from other gpuishell instances and provides reactive state updates
/// through `futures_signals`.
#[derive(Clone)]
pub struct ShellSubscriber {
    data: Mutable<ShellData>,
    #[allow(dead_code)]
    connection: Arc<Mutex<Option<Connection>>>,
}

impl std::fmt::Debug for ShellSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSubscriber")
            .field("data", &self.data)
            .finish()
    }
}

/// Result of trying to acquire the single-instance lock.
pub enum InstanceResult {
    /// This is the primary instance - the service is now running.
    Primary(ShellSubscriber),
    /// Another instance is already running - a message was sent to it.
    Secondary,
    /// Failed to connect to D-Bus.
    Error(String),
}

impl ShellSubscriber {
    /// Try to become the primary instance or signal an existing one.
    ///
    /// If no other instance is running, starts the D-Bus service and returns
    /// `InstanceResult::Primary` with the subscriber.
    ///
    /// If another instance is already running, sends a launcher open request
    /// to it and returns `InstanceResult::Secondary`.
    pub async fn acquire(input: Option<String>) -> InstanceResult {
        // First, check if another instance is running
        if let Some(conn) = try_connect_existing().await {
            // Another instance is running, send the open_launcher message
            let input_str = input.as_deref().unwrap_or("");

            info!(
                "Found existing instance, sending OpenLauncher with input={:?}",
                input_str
            );
            match conn
                .call_method(
                    Some(DBUS_NAME),
                    DBUS_PATH,
                    Some("org.gpuishell.Shell"),
                    "OpenLauncher",
                    &input_str,
                )
                .await
            {
                Ok(_) => {
                    info!("Successfully signaled existing instance to open launcher");
                    return InstanceResult::Secondary;
                }
                Err(e) => {
                    error!("Failed to signal existing instance: {}", e);
                    return InstanceResult::Error(format!(
                        "Failed to signal existing instance: {}",
                        e
                    ));
                }
            }
        }

        // No existing instance, try to become the primary
        let data = Mutable::new(ShellData::default());

        match start_dbus_service(data.clone()).await {
            Ok(conn) => {
                info!("Started D-Bus service as primary instance");

                // If input was provided, queue a launcher request immediately
                if let Some(input_text) = input {
                    data.set(ShellData {
                        launcher_request: Some(LauncherRequest {
                            input: Some(input_text),
                            id: REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst),
                        }),
                    });
                }

                InstanceResult::Primary(ShellSubscriber {
                    data,
                    connection: Arc::new(Mutex::new(Some(conn))),
                })
            }
            Err(e) => {
                error!("Failed to start D-Bus service: {}", e);
                InstanceResult::Error(format!("Failed to start D-Bus service: {}", e))
            }
        }
    }

    /// Get a signal that emits when shell state changes.
    ///
    /// Subscribe to this to react to launcher open requests from other instances.
    pub fn subscribe(&self) -> MutableSignalCloned<ShellData> {
        self.data.signal_cloned()
    }

    /// Get the current shell data snapshot.
    pub fn get(&self) -> ShellData {
        self.data.get_cloned()
    }

    /// Clear the pending launcher request after handling it.
    pub fn clear_request(&self) {
        debug!("Clearing launcher request");
        self.data.set(ShellData {
            launcher_request: None,
        });
    }
}

/// Try to connect to an existing instance's D-Bus service.
async fn try_connect_existing() -> Option<Connection> {
    let conn = Connection::session().await.ok()?;

    // Check if our service name is already owned
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &DBUS_NAME,
        )
        .await
        .ok()?;

    let has_owner: bool = reply.body().deserialize().ok()?;

    if has_owner { Some(conn) } else { None }
}

/// Start the D-Bus service for receiving commands.
async fn start_dbus_service(data: Mutable<ShellData>) -> zbus::Result<Connection> {
    let interface = ShellInterface { data };

    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, interface)?
        .build()
        .await?;

    Ok(connection)
}
