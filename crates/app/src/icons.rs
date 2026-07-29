//! Shell-wide icon vocabulary.
//!
//! Every widget draws from [`ui::IconName`] - the embedded Lucide set - so
//! the shell has no font dependency for its iconography. This module is the
//! one place that maps *service state* (a volume level, a battery charge, a
//! power profile) onto a name, so the bar, the control center and the OSD
//! can't drift apart on what "low battery" looks like.
//!
//! The Lucide set is coarser than the Nerd Font ramps this replaced: there
//! is one `volume` glyph rather than three, and one `sun` rather than a dim
//! ladder. Levels that used to be readable from the glyph now come from the
//! value next to it.

use services::{BatteryData, PowerProfile, ServiceStatus};
use ui::IconName;

// Audio
pub const MICROPHONE: IconName = IconName::Mic;
pub const MICROPHONE_MUTE: IconName = IconName::MicOff;

// Brightness. One `sun` covers every level - the percentage beside it
// carries the detail the old glyph ramp used to.
pub const BRIGHTNESS: IconName = IconName::Sun;

// Connectivity
pub const BLUETOOTH: IconName = IconName::Bluetooth;
pub const BLUETOOTH_OFF: IconName = IconName::BluetoothOff;
pub const BLUETOOTH_CONNECTED: IconName = IconName::BluetoothConnected;
pub const WIFI: IconName = IconName::Wifi;
pub const WIFI_OFF: IconName = IconName::WifiOff;
pub const WIFI_LOCK: IconName = IconName::Lock;
pub const ETHERNET: IconName = IconName::Ethernet;

// Power
pub const POWER_SLEEP: IconName = IconName::Moon;
pub const POWER_BUTTON: IconName = IconName::Power;
pub const CAMERA: IconName = IconName::Camera;
pub const SCREENSHARE: IconName = IconName::ScreenShare;

// UI
pub const CHEVRON_DOWN: IconName = IconName::ChevronDown;
pub const CHEVRON_UP: IconName = IconName::ChevronUp;
pub const CHEVRON_RIGHT: IconName = IconName::ChevronRight;
pub const CHECK: IconName = IconName::Check;
pub const CLOSE: IconName = IconName::Close;
pub const REFRESH: IconName = IconName::Refresh;
pub const LOCK: IconName = IconName::Lock;
pub const TRASH: IconName = IconName::Trash;

/// WiFi icon for a signal strength (0-100).
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
    if muted || level == 0 {
        IconName::VolumeOff
    } else {
        IconName::Volume
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
