//! Shell-wide icon vocabulary.
//!
//! Every widget draws from [`ui::IconName`] - the embedded Lucide set - so
//! the shell has no font dependency for its iconography.
//!
//! What lives here is only the part that takes a *decision*: mapping service
//! state (a volume level, a battery charge, a power profile) onto an icon,
//! so the bar, the control center and the OSD can't drift apart on what
//! "low battery" looks like. Icons that are just themselves - a close button
//! is [`IconName::Close`] - are named at the call site; a `CLOSE = Close`
//! alias would only add a hop.
//!
//! [`ConfigIcon`] is the icon type config files speak, which is a separate
//! concern: it has to tolerate values this set doesn't know.

mod config;

pub use config::{ConfigIcon, deserialize_lenient, source_or};

use services::{BatteryData, PowerProfile, ServiceStatus};
use ui::IconName;

/// Wi-Fi icon for a signal strength (0-100).
pub fn wifi_signal_icon(strength: u8) -> IconName {
    match strength {
        0..=25 => IconName::WifiZero,
        26..=50 => IconName::WifiLow,
        51..=75 => IconName::WifiHigh,
        _ => IconName::Wifi,
    }
}

/// Volume icon for a level (0-100) and mute state.
pub fn volume_icon(level: u8, muted: bool) -> IconName {
    if muted {
        IconName::VolumeOff
    } else {
        match level {
            0 => IconName::VolumeLow,
            1..=50 => IconName::VolumeMedium,
            _ => IconName::Volume,
        }
    }
}

/// Brightness icon for a level (0-100).
pub fn brightness_icon(level: u8) -> IconName {
    match level {
        0..=25 => IconName::SunDim,
        26..=50 => IconName::SunMedium,
        _ => IconName::Sun,
    }
}

/// Battery icon for a charge percentage and charging state.
pub fn battery_icon(percentage: u8, charging: bool) -> IconName {
    if charging {
        IconName::BatteryCharging
    } else if percentage >= 90 {
        IconName::BatteryFull
    } else if percentage >= 60 {
        IconName::BatteryMedium
    } else if percentage >= 20 {
        IconName::BatteryLow
    } else {
        IconName::BatteryWarning
    }
}

/// Battery icon for a UPower reading, or an empty battery when the machine
/// has none.
pub fn battery_data_icon(battery: Option<&BatteryData>) -> IconName {
    match battery {
        Some(b) => battery_icon(b.percentage, b.is_charging()),
        None => IconName::Battery,
    }
}

/// Icon for a power profile.
pub fn power_profile_icon(profile: PowerProfile) -> IconName {
    match profile {
        PowerProfile::Performance => IconName::Zap,
        PowerProfile::PowerSaver => IconName::Moon,
        PowerProfile::Balanced | PowerProfile::Unknown => IconName::Gauge,
    }
}

/// Icon for a service's health, as shown in the `;s` launcher view.
pub fn service_status_icon(status: &ServiceStatus) -> IconName {
    match status {
        ServiceStatus::Active => IconName::CheckCircle,
        ServiceStatus::Initializing => IconName::LoaderCircle,
        ServiceStatus::Error(_) => IconName::XCircle,
        ServiceStatus::Stopped => IconName::Play,
        ServiceStatus::Unavailable => IconName::CircleAlert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each ramp has to actually change icon across its range - one that
    /// returns the same icon everywhere is a constant, not a ramp.
    #[test]
    fn the_level_ramps_are_not_constants() {
        let volume = [0, 25, 100].map(|l| volume_icon(l, false));
        let brightness = [0, 40, 100].map(brightness_icon);
        let wifi = [0, 40, 60, 100].map(wifi_signal_icon);

        for ramp in [&volume[..], &brightness[..], &wifi[..]] {
            let mut seen = ramp.to_vec();
            seen.dedup();
            assert_eq!(seen.len(), ramp.len(), "ramp repeats an icon: {ramp:?}");
        }

        let battery = [0, 30, 70, 100].map(|p| battery_icon(p, false));
        let mut seen = battery.to_vec();
        seen.dedup();
        assert_eq!(
            seen.len(),
            battery.len(),
            "battery ramp repeats: {battery:?}"
        );

        assert_eq!(volume_icon(80, true), IconName::VolumeOff);
        assert_eq!(battery_icon(80, true), IconName::BatteryCharging);
    }
}
