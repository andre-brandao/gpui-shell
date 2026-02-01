use gpui::{div, prelude::*};

pub fn get_battery_percentage() -> Option<u8> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let battery_path = "/sys/class/power_supply/BAT0/capacity";
        if let Ok(contents) = fs::read_to_string(battery_path) {
            if let Ok(percentage) = contents.trim().parse::<u8>() {
                return Some(percentage);
            }
        }
    }
    None
}

pub fn battery(percentage: Option<u8>) -> impl IntoElement {
    let text = percentage.map_or("N/A".to_string(), |p| format!("{}%", p));
    div()
        .flex()
        .items_center()
        .child(format!("Battery: {}", text))
}
