//! Icon constants for Control Center.
//!
//! Uses Nerd Font glyphs.

// Connectivity
pub const WIFI: &str = "󰤨";
pub const WIFI_OFF: &str = "󰤭";
pub const WIFI_WEAK: &str = "󰤟";
pub const WIFI_FAIR: &str = "󰤢";
pub const WIFI_GOOD: &str = "󰤥";
pub const WIFI_STRONG: &str = "󰤨";
pub const WIFI_LOCK: &str = "󰤪";
pub const BLUETOOTH: &str = "󰂯";
pub const BLUETOOTH_CONNECTED: &str = "󰂱";

// Audio / Display
pub const MIC: &str = "󰍬";
pub const SPEAKER: &str = "󰕾";
pub const SPEAKER_MUTED: &str = "󰖁";
pub const SPEAKER_LOW: &str = "󰕿";
pub const SPEAKER_MID: &str = "󰖀";
pub const BRIGHTNESS: &str = "󰃠";
pub const BRIGHTNESS_LOW: &str = "󰃞";
pub const BRIGHTNESS_HIGH: &str = "󰃠";
pub const REFRESH: &str = "⟳";

// Power
pub const POWER: &str = "󰚥";
pub const BATTERY: &str = "󰁹";

// UI
pub const CHECK: &str = "󰄬";

/// Get volume icon based on level and mute state.
pub fn volume_icon(level: u8, muted: bool) -> &'static str {
    if muted {
        SPEAKER_MUTED
    } else if level == 0 {
        SPEAKER_MUTED
    } else if level < 33 {
        SPEAKER_LOW
    } else if level < 66 {
        SPEAKER_MID
    } else {
        SPEAKER
    }
}

/// Get brightness icon based on level (0-100).
pub fn brightness_icon(level: u8) -> &'static str {
    if level < 33 {
        BRIGHTNESS_LOW
    } else if level < 66 {
        BRIGHTNESS
    } else {
        BRIGHTNESS_HIGH
    }
}

/// Get Wi‑Fi icon based on signal strength (0-100).
pub fn wifi_signal_icon(strength: u8) -> &'static str {
    match strength {
        0..=25 => WIFI_WEAK,
        26..=50 => WIFI_FAIR,
        51..=75 => WIFI_GOOD,
        _ => WIFI_STRONG,
    }
}
