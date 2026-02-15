//! Battery widget that displays battery status using the UPower service.

use futures_signals::signal::SignalExt;
use gpui::{Context, Window, div, prelude::*, px};
use services::{BatteryState, UPowerData, UPowerSubscriber};
use ui::{ActiveTheme, font_size, icon_size, spacing};

use crate::config::ActiveConfig;

/// A battery widget that displays the current battery percentage and status.
pub struct Battery {
    data: UPowerData,
}

impl Battery {
    /// Create a new battery widget with a UPower subscriber.
    pub fn new(subscriber: UPowerSubscriber, cx: &mut Context<Self>) -> Self {
        let initial_data = subscriber.get();

        // Subscribe to updates from the UPower service
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let should_continue = this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_ok();

                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();

        Battery { data: initial_data }
    }

    /// Get the battery icon based on current state.
    fn battery_icon(&self) -> &'static str {
        match &self.data.battery {
            Some(battery) => battery.icon(),
            None => "󰂑", // No battery
        }
    }

    /// Get the battery percentage text.
    fn battery_text(&self) -> String {
        match &self.data.battery {
            Some(battery) => format!("{}%", battery.percentage),
            None => String::new(),
        }
    }

    /// Get tooltip text with detailed battery info.
    pub fn tooltip_text(&self) -> String {
        match &self.data.battery {
            Some(battery) => {
                let mut parts = vec![format!("{}%", battery.percentage)];

                let state_str = match battery.state {
                    BatteryState::Charging => "Charging",
                    BatteryState::Discharging => "Discharging",
                    BatteryState::FullyCharged => "Fully charged",
                    BatteryState::Empty => "Empty",
                    BatteryState::PendingCharge => "Pending charge",
                    BatteryState::PendingDischarge => "Pending discharge",
                    BatteryState::Unknown => "Unknown",
                };
                parts.push(state_str.to_string());

                if let Some(time_str) = battery.time_remaining_str() {
                    if battery.is_charging() {
                        parts.push(format!("{} until full", time_str));
                    } else {
                        parts.push(format!("{} remaining", time_str));
                    }
                }

                if let Some(rate) = battery.energy_rate {
                    parts.push(format!("{:.1}W", rate.abs()));
                }

                parts.join(" • ")
            }
            None => "No battery".to_string(),
        }
    }
}

impl Render for Battery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.orientation.is_vertical();

        let icon = self.battery_icon();
        let text = self.battery_text();

        // Get the text color based on battery state
        let text_color = match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    theme.status.error
                } else if battery.is_low() {
                    theme.status.warning
                } else if battery.is_charging() {
                    theme.status.info
                } else {
                    theme.text.primary
                }
            }
            None => theme.text.muted,
        };

        // Get the icon color based on battery state
        let icon_color = match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    theme.status.error
                } else if battery.is_low() {
                    theme.status.warning
                } else if matches!(
                    battery.state,
                    BatteryState::Charging | BatteryState::FullyCharged
                ) {
                    theme.status.success
                } else {
                    theme.text.primary
                }
            }
            None => theme.text.muted,
        };

        div()
            .id("battery-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(spacing::XS))
            // Battery icon
            .child(
                div()
                    .text_size(px(icon_size::MD))
                    .text_color(icon_color)
                    .child(icon),
            )
            // Battery percentage
            .when(!text.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(text_color)
                        .child(text),
                )
            })
    }
}
