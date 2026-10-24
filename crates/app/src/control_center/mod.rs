//! Control Center module for system settings and quick actions.
//!
//! This module provides a panel for controlling system settings like:
//! - WiFi networks and connections
//! - Bluetooth devices
//! - VPN profiles
//! - Volume and brightness
//! - Power profiles and battery status
//!
//! The module is split into submodules for better organization:
//! - `quick_toggles` - Quick toggle buttons for WiFi, Bluetooth, VPN, Mic
//! - `sliders` - Volume and brightness slider controls
//! - `wifi` - WiFi section entity: network list, connect and password flow
//! - `bluetooth` - Bluetooth device list and connections
//! - `vpn` - VPN profile list
//! - `list` - List chrome shared by the WiFi, Bluetooth and VPN sections
//! - `power` - Battery status and power profiles

mod bluetooth;
pub mod config;
mod list;
mod power;
mod quick_toggles;
mod sliders;
mod tooltip;
mod vpn;
mod wifi;

pub use config::{ControlCenterConfig, PowerActionsConfig};

use crate::icons;
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, MouseButton, Task, Window, div, prelude::*, px,
};
use services::{ChargeLimit, UPowerCommand};
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use crate::state::{AppState, watch, watch_task};

pub use quick_toggles::ExpandedSection;
use wifi::WifiSection;

pub const CONTROL_CENTER_PANEL_WIDTH: f32 = 340.0;
/// Cap for an expanded section; the window host also clamps to the output.
pub const CONTROL_CENTER_PANEL_MAX_HEIGHT: f32 = 900.0;

/// Control Center panel component.
///
/// Provides a unified interface for system settings and quick actions.
pub struct ControlCenter {
    /// Currently expanded section
    expanded: ExpandedSection,
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
    /// Present while the WiFi section is expanded.
    wifi: Option<Entity<WifiSection>>,
    /// Signals only the expanded section reads; dropped when it collapses.
    section_watch: Option<Task<()>>,
    /// Take focus back on the next render, e.g. from a closed WiFi prompt.
    refocus: bool,
}

impl ControlCenter {
    /// Create a new control center panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        // Each of these feeds a row shown while every section is collapsed.
        watch(cx, AppState::audio(cx).subscribe(), |_, _, cx| cx.notify());
        watch(cx, AppState::bluetooth(cx).subscribe(), |_, _, cx| {
            cx.notify()
        });
        watch(cx, AppState::brightness(cx).subscribe(), |_, _, cx| {
            cx.notify()
        });
        watch(cx, AppState::network(cx).subscribe(), |_, _, cx| {
            cx.notify()
        });
        watch(cx, AppState::privacy(cx).subscribe(), |_, _, cx| {
            cx.notify()
        });
        watch(cx, AppState::upower(cx).subscribe(), |_, _, cx| cx.notify());

        ControlCenter {
            expanded: ExpandedSection::None,
            focus_handle: cx.focus_handle(),
            wifi: None,
            section_watch: None,
            refocus: false,
        }
    }

    /// Toggle a section's expanded state
    fn toggle_section(&mut self, section: ExpandedSection, cx: &mut Context<Self>) {
        self.expanded = if self.expanded == section {
            ExpandedSection::None
        } else {
            section
        };

        // Dropping the section drops its prompt, and any focus it held.
        self.refocus |= self.wifi.is_some();
        self.wifi = (self.expanded == ExpandedSection::WiFi).then(|| {
            let parent_focus = self.focus_handle.clone();
            cx.new(|cx| WifiSection::new(parent_focus, cx))
        });

        // Per-row progress comes from the queue, not the item list.
        self.section_watch = match self.expanded {
            ExpandedSection::Bluetooth => Some(watch_task(
                cx,
                AppState::bluetooth(cx).pending_signal(),
                |_, _, cx| cx.notify(),
            )),
            ExpandedSection::Vpn => Some(watch_task(
                cx,
                AppState::network(cx).pending_signal(),
                |_, _, cx| cx.notify(),
            )),
            _ => None,
        };
        cx.notify();
    }
}

impl Focusable for ControlCenter {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.refocus) {
            self.focus_handle.focus(window, cx);
        }

        let entity = cx.entity();
        let on_toggle_section = {
            let entity = entity.clone();
            move |section: ExpandedSection, cx: &mut App| {
                entity.update(cx, |this, cx| this.toggle_section(section, cx));
            }
        };
        let on_toggle_section_power = {
            let on_toggle_section = on_toggle_section.clone();
            move |cx: &mut App| on_toggle_section(ExpandedSection::Power, cx)
        };

        let theme = cx.theme();
        let expanded = self.expanded;
        let upower = AppState::upower(cx).get();
        let brightness_state = AppState::brightness(cx).get();
        let show_brightness = brightness_state.max != 0;
        let border_subtle = theme.colors.border_variant;
        let interactive_default = theme.colors.element_background;
        let interactive_hover = theme.colors.element_hover;
        let text_primary = theme.colors.text;
        let text_muted = theme.colors.text_muted;
        let accent_primary = theme.colors.accent;

        let battery = upower.battery.as_ref();
        let battery_icon = icons::battery_data_icon(battery);
        let battery_line = battery
            .map(|b| format!("{}%", b.percentage))
            .unwrap_or_else(|| "AC".to_string());
        let battery_sub = battery
            .map(|b| {
                let status = if let Some(time) = power::format_time_remaining(b) {
                    if b.is_charging() {
                        format!("{} to full", time)
                    } else {
                        format!("{} remaining", time)
                    }
                } else if b.is_charging() {
                    "Charging".to_string()
                } else if b.is_full() {
                    "Fully charged".to_string()
                } else if upower.on_battery == Some(false) {
                    // Charging paused, e.g. held at the charge limit.
                    "Plugged in".to_string()
                } else {
                    "On Battery".to_string()
                };
                match upower.charge_limit {
                    Some(ChargeLimit {
                        enabled: true,
                        end_threshold: Some(end),
                    }) => format!("{status} · limit {end}%"),
                    _ => status,
                }
            })
            .unwrap_or_else(|| "No battery".to_string());
        let charge_limit = upower.charge_limit;
        let upower_service = AppState::upower(cx).clone();
        let profile_service = upower_service.clone();
        let battery_color = if let Some(b) = battery {
            if b.is_critical() {
                theme.colors.status.error
            } else if b.is_charging() {
                theme.colors.status.success
            } else if b.percentage <= 20 {
                theme.colors.status.warning
            } else {
                theme.colors.text
            }
        } else {
            theme.colors.text_muted
        };

        div()
            .id("control-center")
            .track_focus(&self.focus_handle)
            .key_context("ControlCenter")
            .w_full()
            .p(Spacing::Large.pixels())
            .panel_surface(cx)
            .text_color(theme.colors.text)
            .flex()
            .flex_col()
            .gap(Spacing::Large.pixels())
            .child(
                div()
                    .id("control-center-header")
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .px(Spacing::Medium.pixels())
                            .py(Spacing::XSmall.pixels())
                            .panel_card(cx)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap(Spacing::Medium.pixels())
                                    .child(
                                        Icon::new(battery_icon)
                                            .size(IconSize::Medium)
                                            .color(Color::Custom(battery_color)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(TextSize::Small.rems())
                                                    .text_color(text_primary)
                                                    .child(battery_line),
                                            )
                                            .child(
                                                div()
                                                    .text_size(TextSize::XSmall.rems())
                                                    .text_color(text_muted)
                                                    .child(battery_sub),
                                            ),
                                    ),
                            )
                            .when_some(charge_limit, |el, limit| {
                                el.child(
                                    div()
                                        .id("charge-limit-toggle")
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(32.))
                                        .h(px(32.))
                                        .bg(interactive_default)
                                        .border_1()
                                        .border_color(if limit.enabled {
                                            accent_primary
                                        } else {
                                            border_subtle
                                        })
                                        .rounded(Radius::Medium.pixels())
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(interactive_hover))
                                        .on_mouse_down(MouseButton::Left, move |_, _, _| {
                                            upower_service.dispatch(UPowerCommand::SetChargeLimit(
                                                !limit.enabled,
                                            ));
                                        })
                                        .child(
                                            Icon::new(IconName::Heart)
                                                .size(IconSize::XSmall)
                                                .color(if limit.enabled {
                                                    Color::Accent
                                                } else {
                                                    Color::Muted
                                                }),
                                        ),
                                )
                            })
                            .child(
                                div()
                                    .id("power-profile-cycle")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(32.))
                                    .h(px(32.))
                                    .bg(interactive_default)
                                    .border_1()
                                    .border_color(border_subtle)
                                    .rounded(Radius::Medium.pixels())
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(interactive_hover))
                                    .on_mouse_down(MouseButton::Left, move |_, _, _| {
                                        profile_service.dispatch(UPowerCommand::CyclePowerProfile);
                                    })
                                    .child(
                                        Icon::new(icons::power_profile_icon(upower.power_profile))
                                            .size(IconSize::XSmall)
                                            .color(Color::Default),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("power-button")
                            .w(px(36.))
                            .h(px(36.))
                            .rounded(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(if expanded == ExpandedSection::Power {
                                accent_primary
                            } else {
                                border_subtle
                            })
                            .bg(interactive_default)
                            .cursor_pointer()
                            .hover(move |s| s.bg(interactive_hover))
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                on_toggle_section_power(cx);
                            })
                            .child(
                                Icon::new(IconName::Power)
                                    .size(IconSize::Small)
                                    .color(Color::Default),
                            ),
                    ),
            )
            .child(
                div()
                    .id("control-center-volume")
                    .p(Spacing::Medium.pixels())
                    .panel_card(cx)
                    .child(sliders::render_volume_slider(cx)),
            )
            .when(show_brightness, |el| {
                el.child(
                    div()
                        .id("control-center-brightness")
                        .p(Spacing::Medium.pixels())
                        .panel_card(cx)
                        .child(sliders::render_brightness_slider(cx)),
                )
            })
            .child(quick_toggles::render_quick_toggles(
                expanded,
                on_toggle_section,
                cx,
            ))
            .when(expanded != ExpandedSection::None, |el| {
                el.child(
                    div()
                        .id("control-center-dropdown")
                        .w_full()
                        .p(Spacing::Medium.pixels())
                        .panel_card(cx)
                        .flex()
                        .flex_col()
                        .gap(Spacing::Medium.pixels())
                        .when(expanded == ExpandedSection::WiFi, |el| {
                            el.when_some(self.wifi.clone(), |el, wifi| el.child(wifi))
                        })
                        .when(expanded == ExpandedSection::Vpn, |el| {
                            el.child(vpn::render_vpn_section(cx))
                        })
                        .when(expanded == ExpandedSection::Bluetooth, |el| {
                            el.child(bluetooth::render_bluetooth_section(cx))
                        })
                        .when(expanded == ExpandedSection::Power, |el| {
                            el.child(power::render_power_section(cx))
                        }),
                )
            })
    }
}
