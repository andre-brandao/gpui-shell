use std::time::Duration;

use gpui::{Context, Window, div, prelude::*};

pub struct Battery {
    percentage: Option<u8>,
}

impl Battery {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                let percentage = Self::read_battery_percentage();
                let _ = this.update(cx, |this, cx| {
                    this.percentage = percentage;
                    cx.notify();
                });
                cx.background_executor().timer(Duration::from_secs(5)).await;
            }
        })
        .detach();

        Battery {
            percentage: Self::read_battery_percentage(),
        }
    }

    fn read_battery_percentage() -> Option<u8> {
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
}

impl Render for Battery {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let text = self
            .percentage
            .map_or("N/A".to_string(), |p| format!("{}%", p));

        div()
            .flex()
            .items_center()
            .child(format!("Battery: {}", text))
    }
}
