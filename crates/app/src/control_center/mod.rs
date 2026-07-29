//! Control Center module for system settings and quick actions.
//!
//! This module provides a panel for controlling system settings like:
//! - WiFi networks and connections
//! - Bluetooth devices
//! - Volume and brightness
//! - Power profiles and battery status
//!
//! The module is split into submodules for better organization:
//! - `quick_toggles` - Quick toggle buttons for WiFi, Bluetooth, Mic
//! - `sliders` - Volume and brightness slider controls
//! - `wifi` - WiFi network list and password handling
//! - `bluetooth` - Bluetooth device list and connections
//! - `power` - Battery status and power profiles

mod bluetooth;
pub mod config;
mod power;
mod quick_toggles;
mod sliders;
mod tooltip;
mod wifi;

pub use config::{ControlCenterConfig, PowerActionsConfig};

use crate::icons;
use gpui::{
    App, AvailableSpace, Context, FocusHandle, Focusable, MouseButton, Size, Window, div,
    prelude::*, px,
};
use services::{NetworkCommand, UPowerCommand};
use std::rc::Rc;
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorLeft, CursorRight, DeleteWordBack, SelectAll, SelectLeft,
    SelectRight, SelectWordLeft, SelectWordRight, WordLeft, WordRight,
};
use crate::state::{AppState, watch};

pub use quick_toggles::ExpandedSection;
pub use wifi::WifiPasswordState;

pub const CONTROL_CENTER_PANEL_WIDTH: f32 = 340.0;
pub const CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED: f32 = 288.0;

type ToggleSectionCallback = Rc<dyn Fn(ExpandedSection, &mut App)>;
type WifiConnectCallback = Rc<dyn Fn(String, Option<String>, &mut App)>;
type WifiDisconnectCallback = Rc<dyn Fn(String, &mut App)>;

/// Control Center panel component.
///
/// Provides a unified interface for system settings and quick actions.
pub struct ControlCenter {
    /// Currently expanded section (WiFi or Bluetooth)
    expanded: ExpandedSection,
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
    /// Volume slider entity
    /// Brightness slider entity
    /// WiFi password input state
    wifi_password: WifiPasswordState,
}

impl ControlCenter {
    /// Create a new control center panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Subscribe to service updates
        Self::subscribe_to_services(cx);

        ControlCenter {
            expanded: ExpandedSection::None,
            focus_handle,
            wifi_password: WifiPasswordState::default(),
        }
    }

    /// Subscribe to service updates to keep UI in sync
    fn subscribe_to_services(cx: &mut Context<Self>) {
        // Audio
        watch(cx, AppState::audio(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // Bluetooth
        watch(cx, AppState::bluetooth(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // Brightness
        watch(cx, AppState::brightness(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // Network
        watch(cx, AppState::network(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // Privacy
        watch(cx, AppState::privacy(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // UPower
        watch(cx, AppState::upower(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });
    }

    /// Toggle a section's expanded state
    fn toggle_section(&mut self, section: ExpandedSection) {
        if self.expanded == section {
            self.expanded = ExpandedSection::None;
        } else {
            self.expanded = section;
        }
        // Clear password state when switching sections
        self.wifi_password.clear();
    }
}

impl Focusable for ControlCenter {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let network_service = AppState::network(cx).clone();

        // Create entity-based callbacks for section toggling
        let entity = cx.entity().clone();
        let on_toggle_section: ToggleSectionCallback = Rc::new({
            let entity = entity.clone();
            move |section: ExpandedSection, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.toggle_section(section);
                    cx.notify();
                });
            }
        });

        let on_cycle_power_profile: Rc<dyn Fn(&mut App)> = Rc::new({
            let services = AppState::upower(cx).clone();
            move |cx: &mut App| {
                let s = services.clone();
                cx.spawn(async move |_| {
                    let _ = s.dispatch(UPowerCommand::CyclePowerProfile).await;
                })
                .detach();
            }
        });

        // WiFi callbacks
        let wifi_services = network_service.clone();
        let on_wifi_connect: WifiConnectCallback = Rc::new({
            let entity = entity.clone();
            let services = wifi_services.clone();
            move |ssid: String, password: Option<String>, cx: &mut App| {
                let entity = entity.clone();
                let services = services.clone();
                if let Some(pwd) = password {
                    let password = if pwd.is_empty() { None } else { Some(pwd) };

                    entity.update(cx, |this, cx| {
                        this.wifi_password.connecting = true;
                        cx.notify();
                    });

                    cx.spawn({
                        let entity = entity.clone();
                        async move |cx| {
                            let result = services
                                .dispatch(NetworkCommand::Connect { ssid, password })
                                .await;

                            entity.update(cx, |this, cx| {
                                this.wifi_password.connecting = false;
                                if result.is_ok() {
                                    this.wifi_password.clear();
                                } else {
                                    this.wifi_password.error =
                                        Some("Connection failed".to_string());
                                }
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                } else {
                    // Need password - prompt for one
                    entity.update(cx, |this, cx| {
                        this.wifi_password.start_for(ssid);
                        cx.notify();
                    });
                }
            }
        });

        let on_wifi_disconnect: WifiDisconnectCallback = Rc::new({
            let services = wifi_services.clone();
            move |ssid: String, cx: &mut App| {
                let s = services.clone();
                cx.spawn(async move |_| {
                    let _ = s.dispatch(NetworkCommand::Disconnect(ssid)).await;
                })
                .detach();
            }
        });

        let on_cancel_password: Rc<dyn Fn(&mut App)> = Rc::new({
            let entity = entity.clone();
            move |cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.wifi_password.clear();
                    cx.notify();
                });
            }
        });

        let mut desired_width = CONTROL_CENTER_PANEL_WIDTH;
        let mut max_height = None;

        if let Some(display) = window.display(cx) {
            let bounds = display.visible_bounds();
            let visible_width: f32 = bounds.size.width.into();
            let visible_height: f32 = bounds.size.height.into();
            let gutter = Spacing::Medium.value() * 2.0;
            let max_width = (visible_width - gutter).max(240.0);
            let max_height_value = (visible_height - gutter).max(240.0);

            desired_width = desired_width.min(max_width);
            max_height = Some(max_height_value);
        }

        let build_root = |cx: &mut Context<Self>| {
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
                    if let Some(time) = power::format_time_remaining(b) {
                        if b.is_charging() {
                            format!("{} to full", time)
                        } else {
                            format!("{} remaining", time)
                        }
                    } else if b.is_charging() {
                        "Charging".to_string()
                    } else {
                        "On Battery".to_string()
                    }
                })
                .unwrap_or_else(|| "No battery".to_string());
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

            let on_toggle_section_cb = {
                let on_toggle_section = on_toggle_section.clone();
                move |section: ExpandedSection, cx: &mut App| {
                    (on_toggle_section)(section, cx);
                }
            };
            let on_toggle_section_power = {
                let on_toggle_section = on_toggle_section.clone();
                move |cx: &mut App| {
                    (on_toggle_section)(ExpandedSection::Power, cx);
                }
            };
            let on_cycle_power_profile = {
                let on_cycle_power_profile = on_cycle_power_profile.clone();
                move |cx: &mut App| {
                    (on_cycle_power_profile)(cx);
                }
            };
            let on_wifi_connect = {
                let on_wifi_connect = on_wifi_connect.clone();
                move |ssid: String, password: Option<String>, cx: &mut App| {
                    (on_wifi_connect)(ssid, password, cx);
                }
            };
            let on_wifi_disconnect = {
                let on_wifi_disconnect = on_wifi_disconnect.clone();
                move |ssid: String, cx: &mut App| {
                    (on_wifi_disconnect)(ssid, cx);
                }
            };
            let on_cancel_password = {
                let on_cancel_password = on_cancel_password.clone();
                move |cx: &mut App| {
                    (on_cancel_password)(cx);
                }
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
                // Keyboard event handling for password input
                .on_action({
                    let entity = entity.clone();
                    move |_: &Backspace, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.backspace();
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &DeleteWordBack, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.delete_word_back();
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &CursorLeft, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_left(false);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &CursorRight, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_right(false);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &WordLeft, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_word_left(false);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &WordRight, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_word_right(false);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &SelectWordLeft, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_word_left(true);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &SelectWordRight, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_word_right(true);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &SelectLeft, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_left(true);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &SelectRight, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.move_right(true);
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &SelectAll, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.select_all();
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    move |_: &Cancel, _window, cx| {
                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.clear();
                                cx.notify();
                            }
                        });
                    }
                })
                .on_action({
                    let entity = entity.clone();
                    let services = network_service.clone();
                    move |_: &Confirm, _window, cx| {
                        let entity = entity.clone();
                        let services = services.clone();
                        entity.update(cx, |this, cx| {
                            if let Some(ssid) = this.wifi_password.ssid.clone() {
                                let password = this.wifi_password.input.text().to_string();
                                let password = if password.is_empty() {
                                    None
                                } else {
                                    Some(password)
                                };

                                this.wifi_password.connecting = true;
                                cx.notify();

                                cx.spawn({
                                    let entity = cx.entity().clone();
                                    async move |_, cx| {
                                        let result = services
                                            .dispatch(NetworkCommand::Connect { ssid, password })
                                            .await;

                                        entity.update(cx, |this, cx| {
                                            this.wifi_password.connecting = false;
                                            if result.is_ok() {
                                                this.wifi_password.clear();
                                            } else {
                                                this.wifi_password.error =
                                                    Some("Connection failed".to_string());
                                            }
                                            cx.notify();
                                        });
                                    }
                                })
                                .detach();
                            }
                        });
                    }
                })
                .on_key_down({
                    let entity = entity.clone();
                    move |event, _window, cx| {
                        // Handle printable character input for password.
                        if event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                            return;
                        }

                        let input_char = event
                            .keystroke
                            .key_char
                            .as_ref()
                            .and_then(|s| s.chars().next())
                            .or_else(|| {
                                let key = event.keystroke.key.as_str();
                                if key.chars().count() == 1 {
                                    key.chars().next()
                                } else {
                                    None
                                }
                            });

                        let Some(ch) = input_char else {
                            return;
                        };
                        if ch.is_control() {
                            return;
                        }

                        entity.update(cx, |this, cx| {
                            if this.wifi_password.ssid.is_some() {
                                this.wifi_password.input.insert_str(&ch.to_string());
                                cx.notify();
                            }
                        });
                    }
                })
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
                                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                            on_cycle_power_profile(cx);
                                        })
                                        .child(
                                            Icon::new(icons::power_profile_icon(
                                                upower.power_profile,
                                            ))
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
                    on_toggle_section_cb,
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
                                el.child(wifi::render_wifi_section(
                                    &self.wifi_password,
                                    on_wifi_connect,
                                    on_wifi_disconnect,
                                    on_cancel_password,
                                    cx,
                                ))
                            })
                            .when(expanded == ExpandedSection::Bluetooth, |el| {
                                el.child(bluetooth::render_bluetooth_section(cx))
                            })
                            .when(expanded == ExpandedSection::Power, |el| {
                                el.child(power::render_power_section(cx))
                            }),
                    )
                })
        };

        // Measure content to size the panel to its actual height (clamped to display).
        let content_height = {
            let mut measure_root = build_root(cx).into_any_element();
            let available_space = Size {
                width: AvailableSpace::Definite(px(desired_width)),
                height: AvailableSpace::MaxContent,
            };
            measure_root
                .layout_as_root(available_space, window, cx)
                .height
        };

        let mut desired_height = content_height;
        if let Some(max_height_value) = max_height {
            desired_height = desired_height.min(px(max_height_value));
        }

        let desired_size = Size::new(px(desired_width), desired_height);
        if window.viewport_size() != desired_size {
            window.resize(desired_size);
        }

        build_root(cx)
    }
}
