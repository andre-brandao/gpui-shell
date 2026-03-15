//! Brightness service for backlight control.
//!
//! This module provides a reactive subscriber for monitoring and controlling
//! display backlight brightness. Uses udev for device discovery and change
//! monitoring, and D-Bus (systemd-logind) for unprivileged brightness control.

use std::path::{Path, PathBuf};

use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use tokio::io::unix::AsyncFd;
use tracing::{debug, error, info, warn};
use zbus::proxy;

use crate::ServiceStatus;

/// Brightness data state.
#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    /// Current brightness value (raw).
    pub current: u32,
    /// Maximum brightness value.
    pub max: u32,
}

impl BrightnessData {
    /// Get brightness as a percentage (0-100).
    pub fn percentage(&self) -> u8 {
        if self.max == 0 {
            0
        } else {
            ((self.current as f64 / self.max as f64) * 100.0).round() as u8
        }
    }

    /// Get an icon based on brightness level.
    pub fn icon(&self) -> &'static str {
        match self.percentage() {
            0..=25 => "󰃞",
            26..=50 => "󰃟",
            51..=75 => "󰃠",
            _ => "󰃠",
        }
    }
}

/// Commands for controlling brightness.
#[derive(Debug, Clone)]
pub enum BrightnessCommand {
    /// Set brightness to an absolute value.
    Set(u32),
    /// Set brightness as a percentage (0-100).
    SetPercent(u8),
    /// Increase brightness by a percentage.
    Increase(u8),
    /// Decrease brightness by a percentage.
    Decrease(u8),
}

/// Event-driven brightness subscriber.
///
/// This subscriber monitors backlight brightness changes using udev
/// and provides reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct BrightnessSubscriber {
    data: Mutable<BrightnessData>,
    status: Mutable<ServiceStatus>,
    device_path: Option<PathBuf>,
    device_name: Option<String>,
    conn: Option<zbus::Connection>,
}

impl BrightnessSubscriber {
    /// Create a new brightness subscriber and start monitoring.
    ///
    /// Returns Ok even if no backlight device exists (graceful degradation).
    pub async fn new() -> Result<Self> {
        // Find backlight device
        let device_path = find_backlight_device();

        let (data, status, device_name, conn) = match &device_path {
            Some(path) => {
                let brightness_data = read_brightness(path)?;
                let name = path.file_name().and_then(|n| n.to_str()).map(String::from);
                let conn = zbus::Connection::system().await.ok();

                info!(
                    "Brightness service initialized: {} (max: {})",
                    brightness_data.current, brightness_data.max
                );

                (
                    Mutable::new(brightness_data),
                    Mutable::new(ServiceStatus::Active),
                    name,
                    conn,
                )
            }
            None => {
                warn!("No backlight device found");
                (
                    Mutable::new(BrightnessData::default()),
                    Mutable::new(ServiceStatus::Unavailable),
                    None,
                    None,
                )
            }
        };

        let subscriber = Self {
            data: data.clone(),
            status: status.clone(),
            device_path: device_path.clone(),
            device_name,
            conn,
        };

        // Start listener if device exists
        if let Some(path) = device_path {
            start_listener(data, path);
        }

        Ok(subscriber)
    }

    /// Get a signal that emits when brightness changes.
    pub fn subscribe(&self) -> MutableSignalCloned<BrightnessData> {
        self.data.signal_cloned()
    }

    /// Get the current brightness data snapshot.
    pub fn get(&self) -> BrightnessData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }

    /// Check if a backlight device is available.
    pub fn is_available(&self) -> bool {
        self.device_path.is_some()
    }

    /// Execute a brightness command.
    pub async fn dispatch(&self, command: BrightnessCommand) -> Result<()> {
        let (device_name, conn) = match (&self.device_name, &self.conn) {
            (Some(name), Some(conn)) => (name, conn),
            _ => {
                warn!("No backlight device available");
                return Ok(());
            }
        };

        let (max, current) = {
            let data = self.data.lock_ref();
            (data.max, data.current)
        };

        let new_value = match command {
            BrightnessCommand::Set(v) => v.min(max),
            BrightnessCommand::SetPercent(p) => {
                ((p.min(100) as f64 / 100.0) * max as f64).round() as u32
            }
            BrightnessCommand::Increase(p) => {
                let delta = ((p as f64 / 100.0) * max as f64).round() as u32;
                current.saturating_add(delta).min(max)
            }
            BrightnessCommand::Decrease(p) => {
                let delta = ((p as f64 / 100.0) * max as f64).round() as u32;
                current.saturating_sub(delta).max(1) // Don't go to 0
            }
        };

        // Skip if no change needed
        if new_value == current {
            return Ok(());
        }

        debug!(
            "Setting brightness to {} (device: {})",
            new_value, device_name
        );

        let proxy = BrightnessCtrlProxy::new(conn).await?;
        proxy
            .set_brightness("backlight", device_name, new_value)
            .await?;

        // Immediately update internal state (optimistic update)
        // This prevents race conditions when clicking buttons rapidly
        self.data.lock_mut().current = new_value;

        Ok(())
    }
}

// D-Bus proxy for systemd-logind brightness control.
#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    /// Set the brightness of a backlight device.
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> zbus::Result<()>;
}

/// Find the backlight device path using udev.
fn find_backlight_device() -> Option<PathBuf> {
    let mut enumerator = udev::Enumerator::new().ok()?;
    enumerator.match_subsystem("backlight").ok()?;

    enumerator
        .scan_devices()
        .ok()?
        .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
        .map(|d| d.syspath().to_path_buf())
}

/// Read brightness data from sysfs.
fn read_brightness(device_path: &Path) -> Result<BrightnessData> {
    let max = std::fs::read_to_string(device_path.join("max_brightness"))?
        .trim()
        .parse()?;
    let current = std::fs::read_to_string(device_path.join("actual_brightness"))?
        .trim()
        .parse()?;
    Ok(BrightnessData { current, max })
}

/// Start the udev listener task for brightness changes.
fn start_listener(data: Mutable<BrightnessData>, device_path: PathBuf) {
    tokio::task::spawn_blocking(move || {
        let socket = match udev::MonitorBuilder::new()
            .and_then(|b| b.match_subsystem("backlight"))
            .and_then(|b| b.listen())
        {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create udev monitor: {}", e);
                return;
            }
        };

        // Wrap the socket in AsyncFd for tokio async I/O
        let async_socket = match AsyncFd::new(socket) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create async fd: {}", e);
                return;
            }
        };

        let mut current_value = data.lock_ref().current;

        // Use tokio's block_on to run async code in blocking context
        let runtime = tokio::runtime::Handle::current();
        runtime.block_on(async {
            loop {
                // Wait asynchronously until the socket is readable
                let mut guard = match async_socket.readable().await {
                    Ok(g) => g,
                    Err(e) => {
                        error!("Failed to wait for readable: {}", e);
                        break;
                    }
                };

                // Try to read events
                match guard.try_io(|inner| {
                    // Drain all pending events
                    for event in inner.get_ref().iter() {
                        if event.event_type() == udev::EventType::Change {
                            if let Ok(new_data) = read_brightness(&device_path) {
                                if new_data.current != current_value {
                                    current_value = new_data.current;
                                    data.lock_mut().current = new_data.current;
                                    debug!("Brightness changed: {}", new_data.current);
                                }
                            }
                        }
                    }
                    Ok::<(), std::io::Error>(())
                }) {
                    Ok(_) => {}
                    Err(_would_block) => {
                        // False alarm, socket not actually readable yet
                        continue;
                    }
                }
            }
        });
    });
}
