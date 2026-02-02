use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use log::{debug, error, info, warn};
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tokio::io::{unix::AsyncFd, Interest};
use zbus::proxy;

/// Brightness data state.
#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    /// Current brightness value.
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

/// Events from the brightness service.
#[derive(Debug, Clone)]
pub enum BrightnessEvent {
    /// Brightness value changed.
    Changed(u32),
}

/// Commands for the brightness service.
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
    /// Refresh the current brightness value.
    Refresh,
}

/// The brightness service state.
#[derive(Debug, Clone)]
pub struct Brightness {
    pub data: BrightnessData,
    device_path: Option<PathBuf>,
}

impl Default for Brightness {
    fn default() -> Self {
        Self {
            data: BrightnessData::default(),
            device_path: None,
        }
    }
}

impl Deref for Brightness {
    type Target = BrightnessData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for Brightness {
    type UpdateEvent = BrightnessEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            BrightnessEvent::Changed(value) => {
                self.data.current = value;
            }
        }
    }
}

impl Brightness {
    /// Create a new GPUI Entity for the brightness service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<Brightness>>();

        // Spawn a dedicated thread with Tokio runtime for udev/D-Bus operations
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for brightness service");

            rt.block_on(async move {
                if let Err(e) = run_listener(&tx).await {
                    error!("Brightness service failed: {}", e);
                    let _ = tx.send(ServiceEvent::Error(e.to_string()));
                }
            });
        });

        // Poll the channel for updates
        cx.spawn(async move |this, cx| {
            loop {
                let mut last_event = None;
                while let Ok(event) = rx.try_recv() {
                    last_event = Some(event);
                }

                if let Some(event) = last_event {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            match event {
                                ServiceEvent::Init(brightness) => {
                                    this.data = brightness.data;
                                    this.device_path = brightness.device_path;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    error!("Brightness service error: {}", e);
                                }
                            }
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        break;
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        Brightness::default()
    }

    /// Execute a brightness command.
    pub fn dispatch(&self, command: BrightnessCommand, _cx: &mut Context<Self>) {
        let device_path = self.device_path.clone();
        let max = self.data.max;
        let current = self.data.current;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for brightness command");

            rt.block_on(async move {
                if let Err(e) = execute_command(command, device_path, max, current).await {
                    error!("Failed to execute brightness command: {}", e);
                }
            });
        });
    }
}

/// Read the maximum brightness value from sysfs.
fn get_max_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let max_brightness = fs::read_to_string(device_path.join("max_brightness"))?;
    let max_brightness = max_brightness.trim().parse::<u32>()?;
    Ok(max_brightness)
}

/// Read the actual brightness value from sysfs.
fn get_actual_brightness(device_path: &Path) -> anyhow::Result<u32> {
    let actual_brightness = fs::read_to_string(device_path.join("actual_brightness"))?;
    let actual_brightness = actual_brightness.trim().parse::<u32>()?;
    Ok(actual_brightness)
}

/// Enumerate backlight devices using udev.
fn backlight_enumerate() -> anyhow::Result<Vec<udev::Device>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("backlight")?;
    Ok(enumerator.scan_devices()?.collect())
}

/// Find the backlight device path.
fn find_backlight_device() -> anyhow::Result<PathBuf> {
    let devices = backlight_enumerate()?;

    match devices
        .iter()
        .find(|d| d.subsystem().and_then(|s| s.to_str()) == Some("backlight"))
    {
        Some(device) => Ok(device.syspath().to_path_buf()),
        None => {
            warn!("No backlight devices found");
            Err(anyhow::anyhow!("No backlight devices found"))
        }
    }
}

/// Fetch the current brightness data.
fn fetch_brightness_data(device_path: &Path) -> anyhow::Result<BrightnessData> {
    let max = get_max_brightness(device_path)?;
    let current = get_actual_brightness(device_path)?;

    debug!("Max brightness: {max}, current brightness: {current}");

    Ok(BrightnessData { current, max })
}

/// Create a udev monitor for backlight changes.
async fn create_backlight_monitor() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("backlight")?
        .listen()?;

    Ok(AsyncFd::with_interest(
        socket,
        Interest::READABLE | Interest::WRITABLE,
    )?)
}

/// Run the brightness service listener.
async fn run_listener(tx: &mpsc::Sender<ServiceEvent<Brightness>>) -> anyhow::Result<()> {
    // Find backlight device
    let device_path = match find_backlight_device() {
        Ok(path) => path,
        Err(e) => {
            warn!("No backlight device found: {}", e);
            // Send empty init and wait forever (no backlight on this system)
            let _ = tx.send(ServiceEvent::Init(Brightness::default()));
            std::future::pending::<()>().await;
            return Ok(());
        }
    };

    // Send initial state
    let data = fetch_brightness_data(&device_path)?;
    let _ = tx.send(ServiceEvent::Init(Brightness {
        data,
        device_path: Some(device_path.clone()),
    }));

    info!("Brightness service initialized, listening for changes");

    // Set up udev monitor
    let socket = create_backlight_monitor().await?;
    let mut current_value = get_actual_brightness(&device_path).unwrap_or_default();

    loop {
        match socket.writable().await {
            Ok(mut guard) => {
                for evt in guard.get_inner().iter() {
                    debug!("{:?}: {:?}", evt.event_type(), evt.device());

                    if evt.device().subsystem().and_then(|s| s.to_str()) == Some("backlight") {
                        if let udev::EventType::Change = evt.event_type() {
                            debug!("Backlight changed: {:?}", evt.syspath());

                            let new_value =
                                get_actual_brightness(&device_path).unwrap_or_default();

                            if new_value != current_value {
                                current_value = new_value;
                                let _ = tx.send(ServiceEvent::Update(BrightnessEvent::Changed(
                                    new_value,
                                )));
                            }
                        }
                    }
                }
                guard.clear_ready();
            }
            Err(e) => {
                error!("Failed to get writable socket: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Execute a brightness command.
async fn execute_command(
    command: BrightnessCommand,
    device_path: Option<PathBuf>,
    max: u32,
    current: u32,
) -> anyhow::Result<()> {
    let device_path = match device_path {
        Some(path) => path,
        None => {
            warn!("No backlight device available");
            return Ok(());
        }
    };

    let conn = zbus::Connection::system().await?;
    let brightness_ctrl = BrightnessCtrlProxy::new(&conn).await?;

    let device_name = device_path
        .iter()
        .next_back()
        .and_then(|d| d.to_str())
        .unwrap_or_default();

    let new_value = match command {
        BrightnessCommand::Set(value) => value.min(max),
        BrightnessCommand::SetPercent(percent) => {
            ((percent.min(100) as f64 / 100.0) * max as f64).round() as u32
        }
        BrightnessCommand::Increase(percent) => {
            let delta = ((percent as f64 / 100.0) * max as f64).round() as u32;
            current.saturating_add(delta).min(max)
        }
        BrightnessCommand::Decrease(percent) => {
            let delta = ((percent as f64 / 100.0) * max as f64).round() as u32;
            current.saturating_sub(delta).max(1) // Don't go to 0
        }
        BrightnessCommand::Refresh => {
            // Just read the current value, don't set anything
            return Ok(());
        }
    };

    debug!("Setting brightness to {} (device: {})", new_value, device_name);
    brightness_ctrl
        .set_brightness("backlight", device_name, new_value)
        .await?;

    Ok(())
}

// D-Bus proxy for systemd-logind brightness control
#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    /// Set the brightness of a backlight device.
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> zbus::Result<()>;
}
