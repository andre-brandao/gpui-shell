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
pub const BRIGHTNESS: &str = "󰃠";
pub const REFRESH: &str = "⟳";

// Power
pub const POWER: &str = "󰚥";
pub const BATTERY: &str = "󰁹";

// UI
pub const CHECK: &str = "󰄬";

/// Get Wi‑Fi icon based on signal strength (0-100).
pub fn wifi_signal_icon(strength: u8) -> &'static str {
    match strength {
        0..=25 => WIFI_WEAK,
        26..=50 => WIFI_FAIR,
        51..=75 => WIFI_GOOD,
        _ => WIFI_STRONG,
    }
}
