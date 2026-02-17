//! Battery widget that displays battery status using the UPower service.

mod config;
pub use config::BatteryConfig;

use futures_signals::signal::SignalExt;
use gpui::{Context, Window, div, prelude::*, px};
use services::{BatteryState, UPowerData};
use ui::{ActiveTheme, radius};

use super::style;
use crate::config::ActiveConfig;
use crate::state::AppState;

/// A battery widget that displays the current battery percentage and status.
pub struct Battery {
    data: UPowerData,
}

impl Battery {
    /// Create a new battery widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::services(cx).upower.clone();
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
    fn battery_text(&self, is_vertical: bool) -> String {
        match &self.data.battery {
            Some(battery) => style::compact_percent(battery.percentage.into(), is_vertical),
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
        let is_vertical = cx.config().bar.is_vertical();

        let config = &cx.config().bar.modules.battery;
        let icon = if config.show_icon {
            Some(self.battery_icon())
        } else {
            None
        };
        let text = if config.show_percentage {
            self.battery_text(is_vertical)
        } else {
            String::new()
        };
        let icon_size = style::icon(is_vertical);
        let text_size = style::label(is_vertical);

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

        if is_vertical {
            div()
                .id("battery-widget")
                .flex()
                .flex_col()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(true)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                // Battery icon
                .when_some(icon, |el, icon| {
                    el.child(
                        div()
                            .text_size(px(icon_size))
                            .text_color(icon_color)
                            .child(icon),
                    )
                })
                // Battery percentage
                .when(!text.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(px(text_size))
                            .text_color(text_color)
                            .child(text),
                    )
                })
        } else {
            div()
                .id("battery-widget")
                .flex()
                .items_center()
                .gap(px(style::CHIP_GAP))
                .px(px(style::chip_padding_x(false)))
                .py(px(style::CHIP_PADDING_Y))
                .rounded(px(radius::SM))
                // Battery icon
                .when_some(icon, |el, icon| {
                    el.child(
                        div()
                            .text_size(px(icon_size))
                            .text_color(icon_color)
                            .child(icon),
                    )
                })
                // Battery percentage
                .when(!text.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(px(text_size))
                            .text_color(text_color)
                            .child(text),
                    )
                })
        }
    }
}
