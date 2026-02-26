//! UPower service for battery and power profile monitoring.
//!
//! This module provides an event-driven subscriber for monitoring battery status
//! and power profiles via D-Bus. It uses `futures_signals` for reactive state
//! management and supports both read and write operations.

pub mod dbus;

use std::time::Duration;

use anyhow::Result;
use futures_signals::signal::{Mutable, MutableSignalCloned};
use futures_util::StreamExt;
use futures_util::stream::select_all;
use tracing::{debug, error, info, warn};
use zbus::Connection;

use crate::ServiceStatus;
pub use dbus::{BatteryLevel, BatteryState, DeviceType, WarningLevel};
use dbus::{DeviceProxy, PowerProfilesProxy, UPowerService};

/// Power profile (performance/balanced/power-saver).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfile {
    #[default]
    Balanced,
    Performance,
    PowerSaver,
    Unknown,
}

impl From<&str> for PowerProfile {
    fn from(s: &str) -> Self {
        match s {
            "balanced" => PowerProfile::Balanced,
            "performance" => PowerProfile::Performance,
            "power-saver" => PowerProfile::PowerSaver,
            _ => PowerProfile::Unknown,
        }
    }
}

impl PowerProfile {
    /// Get the D-Bus string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerProfile::Balanced => "balanced",
            PowerProfile::Performance => "performance",
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Unknown => "unknown",
        }
    }

    /// Cycle to the next power profile.
    pub fn next(&self) -> Self {
        match self {
            PowerProfile::Balanced => PowerProfile::Performance,
            PowerProfile::Performance => PowerProfile::PowerSaver,
            PowerProfile::PowerSaver => PowerProfile::Balanced,
            PowerProfile::Unknown => PowerProfile::Balanced,
        }
    }

    /// Get a human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Performance => "Performance",
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Unknown => "Unknown",
        }
    }

    /// Get an icon name for this profile.
    pub fn icon(&self) -> &'static str {
        match self {
            PowerProfile::Balanced => "󰗑",
            PowerProfile::Performance => "󱐋",
            PowerProfile::PowerSaver => "󰌪",
            PowerProfile::Unknown => "󰗑",
        }
    }
}

/// Battery data from UPower.
#[derive(Debug, Clone, Default)]
pub struct BatteryData {
    /// Charge percentage (0-100).
    pub percentage: u8,
    /// Current charging/discharging state.
    pub state: BatteryState,
    /// Time remaining until empty (when discharging).
    pub time_to_empty: Option<Duration>,
    /// Time remaining until full (when charging).
    pub time_to_full: Option<Duration>,
    /// Battery health as capacity percentage.
    pub capacity: Option<f64>,
    /// Battery temperature in Celsius.
    pub temperature: Option<f64>,
    /// Energy rate in Watts (positive = discharging, negative = charging).
    pub energy_rate: Option<f64>,
    /// Warning level from UPower.
    pub warning_level: WarningLevel,
    /// Coarse battery level.
    pub battery_level: BatteryLevel,
    /// Icon name suggested by UPower.
    pub icon_name: String,
}

impl BatteryData {
    /// Check if the battery is currently charging.
    pub fn is_charging(&self) -> bool {
        matches!(
            self.state,
            BatteryState::Charging | BatteryState::PendingCharge
        )
    }

    /// Check if the battery is fully charged.
    pub fn is_full(&self) -> bool {
        matches!(self.state, BatteryState::FullyCharged)
    }

    /// Check if the battery is in a critical state.
    pub fn is_critical(&self) -> bool {
        matches!(
            self.warning_level,
            WarningLevel::Critical | WarningLevel::Action
        )
    }

    /// Check if the battery is low.
    pub fn is_low(&self) -> bool {
        matches!(
            self.warning_level,
            WarningLevel::Low | WarningLevel::Critical | WarningLevel::Action
        )
    }

    /// Get a Nerd Font icon for the current battery state.
    pub fn icon(&self) -> &'static str {
        if self.is_charging() {
            match self.percentage {
                0..=10 => "󰢜",
                11..=20 => "󰂆",
                21..=30 => "󰂇",
                31..=40 => "󰂈",
                41..=50 => "󰢝",
                51..=60 => "󰂉",
                61..=70 => "󰢞",
                71..=80 => "󰂊",
                81..=90 => "󰂋",
                _ => "󰂅",
            }
        } else {
            match self.percentage {
                0..=10 => "󰁺",
                11..=20 => "󰁻",
                21..=30 => "󰁼",
                31..=40 => "󰁽",
                41..=50 => "󰁾",
                51..=60 => "󰁿",
                61..=70 => "󰂀",
                71..=80 => "󰂁",
                81..=90 => "󰂂",
                _ => "󰁹",
            }
        }
    }

    /// Format time remaining as a human-readable string.
    pub fn time_remaining_str(&self) -> Option<String> {
        let duration = if self.is_charging() {
            self.time_to_full?
        } else {
            self.time_to_empty?
        };

        let total_mins = duration.as_secs() / 60;
        let hours = total_mins / 60;
        let mins = total_mins % 60;

        if hours > 0 {
            Some(format!("{}h {}m", hours, mins))
        } else {
            Some(format!("{}m", mins))
        }
    }
}

/// Complete UPower service data.
#[derive(Debug, Clone, Default)]
pub struct UPowerData {
    /// Battery data (None if no battery present).
    pub battery: Option<BatteryData>,
    /// Current power profile.
    pub power_profile: PowerProfile,
    /// Whether power profiles daemon is available.
    pub power_profiles_available: bool,
    /// Whether currently running on battery power.
    pub on_battery: bool,
    /// Whether the lid is closed (if applicable).
    pub lid_closed: Option<bool>,
}

impl UPowerData {
    /// Initialize data from D-Bus.
    async fn init(conn: &Connection) -> Result<Self> {
        let upower = UPowerService::new(conn).await?;

        // Get display device (composite battery)
        let device = upower.get_display_device().await?;
        let device_proxy = DeviceProxy::builder(conn)
            .path(device.inner().path())?
            .build()
            .await?;

        // Check if battery is present
        let battery = if device_proxy.is_present().await.unwrap_or(false) {
            Some(Self::fetch_battery_data(&device_proxy).await)
        } else {
            None
        };

        // Get power profile if available
        let (power_profile, power_profiles_available) = match PowerProfilesProxy::new(conn).await {
            Ok(pp) => {
                let profile = pp
                    .active_profile()
                    .await
                    .map(|s| PowerProfile::from(s.as_str()))
                    .unwrap_or_default();
                (profile, true)
            }
            Err(_) => (PowerProfile::default(), false),
        };

        // Get system power state
        let on_battery = upower.on_battery().await.unwrap_or(false);
        let lid_closed = if upower.lid_is_present().await.unwrap_or(false) {
            Some(upower.lid_is_closed().await.unwrap_or(false))
        } else {
            None
        };

        Ok(Self {
            battery,
            power_profile,
            power_profiles_available,
            on_battery,
            lid_closed,
        })
    }

    /// Fetch battery data from device proxy.
    async fn fetch_battery_data(device: &DeviceProxy<'_>) -> BatteryData {
        let percentage = device.percentage().await.unwrap_or(0.0) as u8;
        let state = device.state().await.unwrap_or_default();

        let time_to_empty = device
            .time_to_empty()
            .await
            .ok()
            .filter(|&t| t > 0)
            .map(|t| Duration::from_secs(t as u64));

        let time_to_full = device
            .time_to_full()
            .await
            .ok()
            .filter(|&t| t > 0)
            .map(|t| Duration::from_secs(t as u64));

        let capacity = device.capacity().await.ok().filter(|&c| c > 0.0);
        let temperature = device.temperature().await.ok().filter(|&t| t > 0.0);
        let energy_rate = device.energy_rate().await.ok().filter(|&r| r != 0.0);
        let warning_level = device.warning_level().await.unwrap_or_default();
        let battery_level = device.battery_level().await.unwrap_or_default();
        let icon_name = device.icon_name().await.unwrap_or_default();

        BatteryData {
            percentage,
            state,
            time_to_empty,
            time_to_full,
            capacity,
            temperature,
            energy_rate,
            warning_level,
            battery_level,
            icon_name,
        }
    }
}

/// Commands for controlling the UPower service.
#[derive(Debug, Clone)]
pub enum UPowerCommand {
    /// Set a specific power profile.
    SetPowerProfile(PowerProfile),
    /// Cycle to the next power profile.
    CyclePowerProfile,
    /// Refresh battery data.
    Refresh,
}

/// Event-driven UPower subscriber.
///
/// This subscriber monitors battery status and power profiles via D-Bus signals,
/// providing reactive state updates through `futures_signals`.
#[derive(Debug, Clone)]
pub struct UPowerSubscriber {
    data: Mutable<UPowerData>,
    status: Mutable<ServiceStatus>,
    conn: Connection,
}

impl UPowerSubscriber {
    /// Create a new UPower subscriber and start monitoring.
    pub async fn new() -> Result<Self> {
        let conn = Connection::system().await?;
        let status = Mutable::new(ServiceStatus::Initializing);

        let data = match UPowerData::init(&conn).await {
            Ok(d) => {
                status.set(ServiceStatus::Active);
                d
            }
            Err(e) => {
                error!("Failed to initialize UPower data: {}", e);
                status.set(ServiceStatus::Error(None));
                UPowerData::default()
            }
        };

        let data = Mutable::new(data);

        let subscriber = Self {
            data: data.clone(),
            status: status.clone(),
            conn: conn.clone(),
        };

        // Spawn the monitoring task
        let sub_for_task = subscriber.clone();
        tokio::spawn(async move {
            if let Err(e) = sub_for_task.run().await {
                error!("UPower subscriber error: {}", e);
                *sub_for_task.status.lock_mut() = ServiceStatus::Error(None);
            }
        });

        Ok(subscriber)
    }

    /// Get a signal that emits when data changes.
    pub fn subscribe(&self) -> MutableSignalCloned<UPowerData> {
        self.data.signal_cloned()
    }

    /// Get the current data snapshot.
    pub fn get(&self) -> UPowerData {
        self.data.get_cloned()
    }

    /// Get the current service status.
    pub fn status(&self) -> ServiceStatus {
        self.status.get_cloned()
    }

    /// Execute a command.
    pub async fn dispatch(&self, command: UPowerCommand) -> Result<()> {
        match command {
            UPowerCommand::SetPowerProfile(profile) => {
                self.set_power_profile(profile).await?;
            }
            UPowerCommand::CyclePowerProfile => {
                let current = self.data.lock_ref().power_profile;
                self.set_power_profile(current.next()).await?;
            }
            UPowerCommand::Refresh => {
                self.refresh().await?;
            }
        }
        Ok(())
    }

    /// Set the power profile.
    async fn set_power_profile(&self, profile: PowerProfile) -> Result<()> {
        let pp = PowerProfilesProxy::new(&self.conn).await?;
        pp.set_active_profile(profile.as_str()).await?;
        debug!("Set power profile to: {:?}", profile);
        Ok(())
    }

    /// Refresh all data from D-Bus.
    async fn refresh(&self) -> Result<()> {
        let new_data = UPowerData::init(&self.conn).await?;
        self.data.set(new_data);
        debug!("Refreshed UPower data");
        Ok(())
    }

    /// Run the event monitoring loop.
    async fn run(&self) -> Result<()> {
        info!("UPower subscriber started");

        let upower = UPowerService::new(&self.conn).await?;
        let device = upower.get_display_device().await?;
        let device_proxy = DeviceProxy::builder(&self.conn)
            .path(device.inner().path())?
            .build()
            .await?;

        // Create streams for battery property changes
        let data = self.data.clone();

        let percentage_stream = device_proxy
            .receive_percentage_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("Battery percentage changed: {}%", value);
                            Self::update_battery_field(&data, |b| {
                                b.percentage = value as u8;
                            });
                        }
                    }
                }
            })
            .boxed();

        let state_stream = device_proxy
            .receive_state_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("Battery state changed: {:?}", value);
                            Self::update_battery_field(&data, |b| {
                                b.state = value;
                            });
                        }
                    }
                }
            })
            .boxed();

        let time_to_empty_stream = device_proxy
            .receive_time_to_empty_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("Time to empty changed: {}s", value);
                            Self::update_battery_field(&data, |b| {
                                b.time_to_empty = if value > 0 {
                                    Some(Duration::from_secs(value as u64))
                                } else {
                                    None
                                };
                            });
                        }
                    }
                }
            })
            .boxed();

        let time_to_full_stream = device_proxy
            .receive_time_to_full_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("Time to full changed: {}s", value);
                            Self::update_battery_field(&data, |b| {
                                b.time_to_full = if value > 0 {
                                    Some(Duration::from_secs(value as u64))
                                } else {
                                    None
                                };
                            });
                        }
                    }
                }
            })
            .boxed();

        let warning_stream = device_proxy
            .receive_warning_level_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("Warning level changed: {:?}", value);
                            Self::update_battery_field(&data, |b| {
                                b.warning_level = value;
                            });
                        }
                    }
                }
            })
            .boxed();

        // Create stream for UPower on_battery changes
        let on_battery_stream = upower
            .receive_on_battery_changed()
            .await
            .then({
                let data = data.clone();
                move |change| {
                    let data = data.clone();
                    async move {
                        if let Ok(value) = change.get().await {
                            debug!("On battery changed: {}", value);
                            data.lock_mut().on_battery = value;
                        }
                    }
                }
            })
            .boxed();

        // Create stream for power profile changes (if available)
        let profile_stream = match PowerProfilesProxy::new(&self.conn).await {
            Ok(pp) => pp
                .receive_active_profile_changed()
                .await
                .then({
                    let data = data.clone();
                    move |change| {
                        let data = data.clone();
                        async move {
                            if let Ok(value) = change.get().await {
                                let profile = PowerProfile::from(value.as_str());
                                debug!("Power profile changed: {:?}", profile);
                                data.lock_mut().power_profile = profile;
                            }
                        }
                    }
                })
                .boxed(),
            Err(e) => {
                warn!("PowerProfiles not available: {}", e);
                // Return a stream that never yields
                futures_util::stream::pending().boxed()
            }
        };

        // Combine all streams
        let mut events = select_all(vec![
            percentage_stream,
            state_stream,
            time_to_empty_stream,
            time_to_full_stream,
            warning_stream,
            on_battery_stream,
            profile_stream,
        ]);

        // Process events
        while (events.next().await).is_some() {
            // Events are processed in their respective handlers
        }

        warn!("UPower event stream ended unexpectedly");
        Ok(())
    }

    /// Helper to update a single battery field.
    fn update_battery_field<F>(data: &Mutable<UPowerData>, updater: F)
    where
        F: FnOnce(&mut BatteryData),
    {
        let mut guard = data.lock_mut();
        if let Some(ref mut battery) = guard.battery {
            updater(battery);
        }
        // If battery is None, we ignore the update - a full refresh would be needed
    }
}
