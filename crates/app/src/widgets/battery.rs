//! Battery widget that displays percentage and status colors.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, Hsla, Window, div, prelude::*, px, rems};
use services::{BatteryState, Services, UPowerData, UPowerSubscriber};
use ui::prelude::*;

pub struct Battery {
    data: UPowerData,
}

impl Battery {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let subscriber: UPowerSubscriber = services.upower.clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the UPower service
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                while let Some(data) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            }
        })
        .detach();

        Battery { data: initial_data }
    }

    fn battery_icon(&self) -> &'static str {
        match &self.data.battery {
            Some(battery) => battery.icon(),
            None => "󰂑", // No battery
        }
    }

    fn battery_text(&self) -> String {
        match &self.data.battery {
            Some(battery) => format!("{}%", battery.percentage),
            None => String::new(),
        }
    }

    fn text_color(&self, cx: &Context<Self>) -> Hsla {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    status.error
                } else if battery.is_low() {
                    status.warning
                } else if battery.is_charging() {
                    status.info
                } else {
                    colors.text
                }
            }
            None => colors.text_muted,
        }
    }

    fn icon_color(&self, cx: &Context<Self>) -> Hsla {
        let colors = cx.theme().colors();
        let status = cx.theme().status();
        match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    status.error
                } else if battery.is_low() {
                    status.warning
                } else if matches!(
                    battery.state,
                    BatteryState::Charging | BatteryState::FullyCharged
                ) {
                    status.success
                } else {
                    colors.text
                }
            }
            None => colors.text_muted,
        }
    }
}

impl Render for Battery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let icon = self.battery_icon();
        let text = self.battery_text();
        let text_color = self.text_color(cx);
        let icon_color = self.icon_color(cx);

        div()
            .id("battery-widget")
            .flex()
            .items_center()
            .gap(px(4.0))
            // Battery icon
            .child(
                div()
                    .text_size(rems(0.85))
                    .text_color(icon_color)
                    .child(icon),
            )
            // Battery percentage
            .when(!text.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(rems(0.78))
                        .text_color(text_color)
                        .child(text),
                )
            })
    }
}
