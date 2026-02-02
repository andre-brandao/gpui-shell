mod dbus;

use crate::services::{ReadOnlyService, ServiceEvent};
use gpui::Context;
use std::ops::Deref;
use std::sync::mpsc;
use std::time::Duration;

/// Battery charging status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl Default for BatteryStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

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
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerProfile::Balanced => "balanced",
            PowerProfile::Performance => "performance",
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Unknown => "unknown",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PowerProfile::Balanced => PowerProfile::Performance,
            PowerProfile::Performance => PowerProfile::PowerSaver,
            PowerProfile::PowerSaver => PowerProfile::Balanced,
            PowerProfile::Unknown => PowerProfile::Balanced,
        }
    }
}

/// Battery data from UPower.
#[derive(Debug, Clone, Default)]
pub struct BatteryData {
    pub percentage: u8,
    pub status: BatteryStatus,
    pub time_to_empty: Option<Duration>,
    pub time_to_full: Option<Duration>,
}

/// UPower service data.
#[derive(Debug, Clone, Default)]
pub struct UPowerData {
    pub battery: Option<BatteryData>,
    pub power_profile: PowerProfile,
}

/// Events from UPower service.
#[derive(Debug, Clone)]
pub enum UPowerEvent {
    BatteryChanged(Option<BatteryData>),
    PowerProfileChanged(PowerProfile),
}

/// Commands for UPower service.
#[derive(Debug, Clone)]
pub enum UPowerCommand {
    SetPowerProfile(PowerProfile),
    CyclePowerProfile,
}

/// UPower service for battery and power profile monitoring.
#[derive(Debug, Clone, Default)]
pub struct UPower {
    pub data: UPowerData,
}

impl Deref for UPower {
    type Target = UPowerData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ReadOnlyService for UPower {
    type UpdateEvent = UPowerEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UPowerEvent::BatteryChanged(battery) => {
                self.data.battery = battery;
            }
            UPowerEvent::PowerProfileChanged(profile) => {
                self.data.power_profile = profile;
            }
        }
    }
}

impl UPower {
    /// Create a new GPUI Entity for the UPower service.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (tx, rx) = mpsc::channel::<ServiceEvent<UPower>>();

        // Spawn a dedicated thread with Tokio runtime for D-Bus operations
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for UPower service");

            rt.block_on(async move {
                if let Err(e) = dbus::run_listener(&tx).await {
                    log::error!("UPower service failed: {}", e);
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
                                ServiceEvent::Init(upower) => {
                                    this.data = upower.data;
                                }
                                ServiceEvent::Update(update_event) => {
                                    this.update(update_event);
                                }
                                ServiceEvent::Error(e) => {
                                    log::error!("UPower service error: {}", e);
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
                    .timer(std::time::Duration::from_millis(100))
                    .await;
            }
        })
        .detach();

        UPower::default()
    }

    /// Execute a UPower command.
    pub fn dispatch(&mut self, command: UPowerCommand, _cx: &mut Context<Self>) {
        let current_profile = self.data.power_profile;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for UPower command");

            rt.block_on(async move {
                if let Err(e) = dbus::execute_command(command, current_profile).await {
                    log::error!("Failed to execute UPower command: {}", e);
                }
            });
        });
    }
}
