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

use crate::lifecycle::{Lifecycle, ManagedService, RunToken};

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
#[derive(Debug, Clone, Default)]
pub struct BrightnessSubscriber {
    data: Mutable<BrightnessData>,
    lifecycle: Lifecycle,
    /// Backlight device found by the last start, if any.
    device: Mutable<Option<PathBuf>>,
}

impl BrightnessSubscriber {
    /// Create a new brightness subscriber. Device discovery and monitoring
    /// happen in [`ManagedService::start`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a signal that emits when brightness changes.
    pub fn subscribe(&self) -> MutableSignalCloned<BrightnessData> {
        self.data.signal_cloned()
    }

    /// Get the current brightness data snapshot.
    pub fn get(&self) -> BrightnessData {
        self.data.get_cloned()
    }

    /// Check if a backlight device is available.
    pub fn is_available(&self) -> bool {
        self.device.lock_ref().is_some()
    }

    /// Execute a brightness command.
    pub async fn dispatch(&self, command: BrightnessCommand) -> Result<()> {
        let device_name = self
            .device
            .lock_ref()
            .as_ref()
            .and_then(|path| path.file_name()?.to_str().map(String::from));
        let Some(device_name) = device_name else {
            warn!("No backlight device available");
            return Ok(());
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

        let conn = crate::bus::system().await?;
        let proxy = BrightnessCtrlProxy::new(&conn).await?;
        proxy
            .set_brightness("backlight", &device_name, new_value)
            .await?;

        // Immediately update internal state (optimistic update)
        // This prevents race conditions when clicking buttons rapidly
        self.data.lock_mut().current = new_value;

        Ok(())
    }
}

impl ManagedService for BrightnessSubscriber {
    fn name(&self) -> &'static str {
        "Brightness"
    }

    fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    fn memory_bytes(&self) -> usize {
        size_of::<BrightnessData>()
            + self
                .device
                .lock_ref()
                .as_ref()
                .map_or(0, |path| path.as_os_str().len())
    }

    fn start(&self) {
        let token = self.lifecycle.begin();

        let Some(device_path) = find_backlight_device() else {
            warn!("No backlight device found");
            self.device.set(None);
            self.lifecycle.set_unavailable();
            return;
        };

        match read_brightness(&device_path) {
            Ok(data) => {
                info!(
                    "Brightness service initialized: {} (max: {})",
                    data.current, data.max
                );
                self.data.set(data);
            }
            Err(e) => {
                error!("Failed to read brightness: {}", e);
                token.error(e.to_string());
                return;
            }
        }

        self.device.set(Some(device_path.clone()));
        token.active();
        start_listener(self.data.clone(), device_path, token);
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
fn start_listener(data: Mutable<BrightnessData>, device_path: PathBuf, token: RunToken) {
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
            while token.alive() {
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
                        if event.event_type() == udev::EventType::Change
                            && let Ok(new_data) = read_brightness(&device_path)
                            && new_data.current != current_value
                        {
                            current_value = new_data.current;
                            data.lock_mut().current = new_data.current;
                            debug!("Brightness changed: {}", new_data.current);
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
