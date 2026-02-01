use super::{BatteryData, BatteryStatus, PowerProfile, UPower, UPowerCommand, UPowerEvent};
use crate::services::ServiceEvent;
use anyhow::Result;
use futures_lite::StreamExt;
use std::sync::mpsc;
use std::time::Duration;
use zbus::proxy;

// UPower D-Bus proxy
#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    fn get_display_device(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

// UPower Device D-Bus proxy
#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait UPowerDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;
}

// Power Profiles Daemon D-Bus proxy
#[proxy(
    interface = "net.hadess.PowerProfiles",
    default_service = "net.hadess.PowerProfiles",
    default_path = "/net/hadess/PowerProfiles"
)]
trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
}

pub async fn run_listener(tx: &mpsc::Sender<ServiceEvent<UPower>>) -> Result<()> {
    let conn = zbus::Connection::system().await?;

    // Get initial battery state
    let battery = fetch_battery_data(&conn).await.ok().flatten();
    let _ = tx.send(ServiceEvent::Update(UPowerEvent::BatteryChanged(battery)));

    // Get initial power profile
    if let Ok(profile) = fetch_power_profile(&conn).await {
        let _ = tx.send(ServiceEvent::Update(UPowerEvent::PowerProfileChanged(
            profile,
        )));
    }

    // Get display device path for monitoring
    let upower = UPowerProxy::new(&conn).await?;
    let device_path = upower.get_display_device().await?;
    let device = UPowerDeviceProxy::builder(&conn)
        .path(device_path)?
        .build()
        .await?;

    // Monitor battery changes
    let mut percentage_stream = device.receive_percentage_changed().await;
    let mut state_stream = device.receive_state_changed().await;

    // Monitor power profile changes
    let power_profiles = PowerProfilesProxy::new(&conn).await;
    let mut profile_stream = if let Ok(ref pp) = power_profiles {
        Some(pp.receive_active_profile_changed().await)
    } else {
        None
    };

    loop {
        tokio::select! {
            Some(_) = percentage_stream.next() => {
                if let Ok(battery) = fetch_battery_data(&conn).await {
                    let _ = tx.send(ServiceEvent::Update(UPowerEvent::BatteryChanged(battery)));
                }
            }
            Some(_) = state_stream.next() => {
                if let Ok(battery) = fetch_battery_data(&conn).await {
                    let _ = tx.send(ServiceEvent::Update(UPowerEvent::BatteryChanged(battery)));
                }
            }
            Some(_) = async {
                match &mut profile_stream {
                    Some(stream) => stream.next().await,
                    None => std::future::pending().await,
                }
            } => {
                if let Ok(profile) = fetch_power_profile(&conn).await {
                    let _ = tx.send(ServiceEvent::Update(UPowerEvent::PowerProfileChanged(profile)));
                }
            }
        }
    }
}

async fn fetch_battery_data(conn: &zbus::Connection) -> Result<Option<BatteryData>> {
    let upower = UPowerProxy::new(conn).await?;
    let device_path = upower.get_display_device().await?;

    let device = UPowerDeviceProxy::builder(conn)
        .path(device_path)?
        .build()
        .await?;

    // Check if battery is present
    if !device.is_present().await.unwrap_or(false) {
        return Ok(None);
    }

    let percentage = device.percentage().await.unwrap_or(0.0) as u8;
    let state = match device.state().await.unwrap_or(0) {
        1 => BatteryStatus::Charging,
        2 => BatteryStatus::Discharging,
        4 => BatteryStatus::Full,
        6 => BatteryStatus::NotCharging,
        _ => BatteryStatus::Unknown,
    };

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

    Ok(Some(BatteryData {
        percentage,
        status: state,
        time_to_empty,
        time_to_full,
    }))
}

async fn fetch_power_profile(conn: &zbus::Connection) -> Result<PowerProfile> {
    let power_profiles = PowerProfilesProxy::new(conn).await?;
    let profile = power_profiles.active_profile().await?;
    Ok(PowerProfile::from(profile.as_str()))
}

pub async fn execute_command(command: UPowerCommand, current_profile: PowerProfile) -> Result<()> {
    let conn = zbus::Connection::system().await?;
    let power_profiles = PowerProfilesProxy::new(&conn).await?;

    match command {
        UPowerCommand::SetPowerProfile(profile) => {
            power_profiles.set_active_profile(profile.as_str()).await?;
        }
        UPowerCommand::CyclePowerProfile => {
            let next = current_profile.next();
            power_profiles.set_active_profile(next.as_str()).await?;
        }
    }

    Ok(())
}
