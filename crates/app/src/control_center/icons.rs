//! Icon constants for the Control Center.
//!
//! Uses Nerd Font glyphs for consistent iconography.

// Audio
pub const VOLUME_HIGH: &str = "󰕾";
pub const VOLUME_MED: &str = "󰖀";
pub const VOLUME_LOW: &str = "󰕿";
pub const VOLUME_MUTE: &str = "󰝟";
pub const MICROPHONE: &str = "󰍬";
pub const MICROPHONE_MUTE: &str = "󰍭";

// Brightness
pub const BRIGHTNESS: &str = "󰃟";
pub const BRIGHTNESS_LOW: &str = "󰃞";
pub const BRIGHTNESS_HIGH: &str = "󰃠";

// Connectivity
pub const BLUETOOTH: &str = "󰂯";
pub const BLUETOOTH_OFF: &str = "󰂲";
pub const BLUETOOTH_CONNECTED: &str = "󰂱";
pub const WIFI: &str = "󰤨";
pub const WIFI_OFF: &str = "󰤭";
pub const WIFI_WEAK: &str = "󰤟";
pub const WIFI_FAIR: &str = "󰤢";
pub const WIFI_GOOD: &str = "󰤥";
pub const WIFI_STRONG: &str = "󰤨";
pub const WIFI_LOCK: &str = "󰤪";

// Power
pub const BATTERY_FULL: &str = "󰁹";
pub const BATTERY_HIGH: &str = "󰂁";
pub const BATTERY_MED: &str = "󰁿";
pub const BATTERY_LOW: &str = "󰁻";
pub const BATTERY_CRITICAL: &str = "󰂃";
pub const BATTERY_CHARGING: &str = "󰂄";
pub const POWER_PROFILE: &str = "󰌪";
pub const POWER_SAVER: &str = "󰌪";
pub const POWER_BALANCED: &str = "󰛲";
pub const POWER_PERFORMANCE: &str = "󱐋";
pub const POWER_SLEEP: &str = "󰒲";
pub const POWER_BUTTON: &str = "⏻";
pub const CAMERA: &str = "󰄀";

// UI
pub const CHEVRON_DOWN: &str = "󰅀";
pub const CHEVRON_UP: &str = "󰅃";
pub const CHEVRON_RIGHT: &str = "󰅂";
pub const CHECK: &str = "󰄬";
pub const CLOSE: &str = "󰅖";
pub const SETTINGS: &str = "󰒓";
pub const REFRESH: &str = "󰑓";
pub const LOCK: &str = "󰌾";
pub const SIGNAL_STRENGTH: &str = "󰣺";
pub const TRASH: &str = "󰆴";

/// Get WiFi icon based on signal strength (0-100)
pub fn wifi_signal_icon(strength: u8) -> &'static str {
    match strength {
        0..=25 => WIFI_WEAK,
        26..=50 => WIFI_FAIR,
        51..=75 => WIFI_GOOD,
        _ => WIFI_STRONG,
    }
}

/// Get volume icon based on level (0-100) and mute state
pub fn volume_icon(level: u8, muted: bool) -> &'static str {
    if muted || level == 0 {
        VOLUME_MUTE
    } else if level >= 66 {
        VOLUME_HIGH
    } else if level >= 33 {
        VOLUME_MED
    } else {
        VOLUME_LOW
    }
}

/// Get battery icon based on percentage and charging state
pub fn battery_icon(percentage: u8, charging: bool) -> &'static str {
    if charging {
        BATTERY_CHARGING
    } else if percentage >= 90 {
        BATTERY_FULL
    } else if percentage >= 60 {
        BATTERY_HIGH
    } else if percentage >= 30 {
        BATTERY_MED
    } else if percentage >= 10 {
        BATTERY_LOW
    } else {
        BATTERY_CRITICAL
    }
}

/// Get power profile icon
pub fn power_profile_icon(profile: &str) -> &'static str {
    match profile {
        "power-saver" => POWER_SAVER,
        "performance" => POWER_PERFORMANCE,
        _ => POWER_BALANCED,
    }
}
