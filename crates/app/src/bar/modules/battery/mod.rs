//! Battery widget that displays battery status using the UPower service.

mod config;
pub use config::BatteryConfig;

use gpui::{AnyElement, Context, Window, div, prelude::*, px};
use services::{BatteryState, UPowerData};
use ui::{ActiveTheme, Color, Icon, IconName};

use super::{BarWidget, style};
use crate::config::ActiveConfig;
use crate::state::AppState;
use crate::state::watch;

/// A battery widget that displays the current battery percentage and status.
pub struct Battery {
    data: UPowerData,
}

struct BatteryView {
    icon: Option<IconName>,
    text: String,
    icon_color: gpui::Hsla,
    text_color: gpui::Hsla,
}

impl Battery {
    /// Create a new battery widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::upower(cx).clone();
        let initial_data = subscriber.get();

        // Subscribe to updates from the UPower service
        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Battery { data: initial_data }
    }

    /// Get the battery icon based on current state.
    fn battery_icon(&self) -> IconName {
        crate::icons::battery_data_icon(self.data.battery.as_ref())
    }

    /// Get the battery percentage text.
    fn battery_text(&self, is_vertical: bool) -> String {
        match &self.data.battery {
            Some(battery) => style::compact_percent(battery.percentage.into(), is_vertical),
            None => String::new(),
        }
    }

    fn icon_color(&self, theme: &ui::Theme) -> gpui::Hsla {
        match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    theme.colors.status.error
                } else if battery.is_low() {
                    theme.colors.status.warning
                } else if matches!(
                    battery.state,
                    BatteryState::Charging | BatteryState::FullyCharged
                ) {
                    theme.colors.status.success
                } else {
                    theme.colors.text
                }
            }
            None => theme.colors.text_muted,
        }
    }

    fn text_color(&self, theme: &ui::Theme) -> gpui::Hsla {
        match &self.data.battery {
            Some(battery) => {
                if battery.is_critical() {
                    theme.colors.status.error
                } else if battery.is_low() {
                    theme.colors.status.warning
                } else if battery.is_charging() {
                    theme.colors.status.info
                } else {
                    theme.colors.text
                }
            }
            None => theme.colors.text_muted,
        }
    }

    fn view(&self, theme: &ui::Theme, is_vertical: bool, config: &BatteryConfig) -> BatteryView {
        BatteryView {
            icon: config.show_icon.then(|| self.battery_icon()),
            text: if config.show_percentage {
                self.battery_text(is_vertical)
            } else {
                String::new()
            },
            icon_color: self.icon_color(theme),
            text_color: self.text_color(theme),
        }
    }

    fn render_battery_content(
        &self,
        _theme: &ui::Theme,
        view: BatteryView,
        is_vertical: bool,
    ) -> AnyElement {
        div()
            .id("battery-widget")
            .flex()
            .when(is_vertical, |el| el.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .when_some(view.icon, |el, icon| {
                el.child(
                    Icon::new(icon)
                        .size(style::icon(is_vertical))
                        .color(Color::Custom(view.icon_color)),
                )
            })
            .when(!view.text.is_empty(), |this| {
                this.child(if is_vertical {
                    style::vertical_text_line(
                        div()
                            .text_size(style::label_size(is_vertical).rems())
                            .text_color(view.text_color)
                            .child(view.text),
                    )
                } else {
                    div()
                        .text_size(style::label_size(is_vertical).rems())
                        .text_color(view.text_color)
                        .child(view.text)
                        .into_any_element()
                })
            })
            .into_any_element()
    }
}

impl BarWidget for Battery {
    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().bar.modules.battery;
        self.render_battery_content(theme, self.view(theme, true, config), true)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().bar.modules.battery;
        self.render_battery_content(theme, self.view(theme, false, config), false)
    }
}

impl Render for Battery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}
