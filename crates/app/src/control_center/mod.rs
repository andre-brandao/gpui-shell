//! Control Center module for system settings and quick actions.
//!
//! This module provides a panel for controlling system settings like:
//! - WiFi networks and connections
//! - Bluetooth devices
//! - Volume and brightness
//! - Power profiles and battery status
//!
//! The module is split into submodules for better organization:
//! - `icons` - Icon constants (Nerd Font glyphs)
//! - `quick_toggles` - Quick toggle buttons for WiFi, Bluetooth, Mic
//! - `sliders` - Volume and brightness slider controls
//! - `wifi` - WiFi network list and password handling
//! - `bluetooth` - Bluetooth device list and connections
//! - `power` - Battery status and power profiles

mod bluetooth;
pub mod config;
pub mod icons;
mod power;
mod quick_toggles;
mod sliders;
mod wifi;

pub use config::{ControlCenterConfig, PowerActionsConfig};

use gpui::{
    App, AvailableSpace, Context, Entity, FocusHandle, Focusable, MouseButton, Size, Window, div,
    prelude::*, px,
};
use services::{AudioCommand, BrightnessCommand, NetworkCommand, UPowerCommand};
use std::rc::Rc;
use ui::{ActiveTheme, Slider, SliderEvent, font_size, icon_size, radius, spacing};

use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorLeft, CursorRight, DeleteWordBack, SelectAll, SelectLeft,
    SelectRight, SelectWordLeft, SelectWordRight, WordLeft, WordRight,
};
use crate::state::{AppState, watch};

pub use quick_toggles::ExpandedSection;
pub use wifi::WifiPasswordState;

pub const CONTROL_CENTER_PANEL_WIDTH: f32 = 340.0;
pub const CONTROL_CENTER_PANEL_HEIGHT_COLLAPSED: f32 = 288.0;

/// Control Center panel component.
///
/// Provides a unified interface for system settings and quick actions.
pub struct ControlCenter {
    /// Currently expanded section (WiFi or Bluetooth)
    expanded: ExpandedSection,
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
    /// Volume slider entity
    volume_slider: Entity<Slider>,
    /// Brightness slider entity
    brightness_slider: Entity<Slider>,
    /// WiFi password input state
    wifi_password: WifiPasswordState,
}

impl ControlCenter {
    /// Create a new control center panel.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Create volume slider
        let audio = AppState::audio(cx).get();
        let volume_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(audio.sink_volume as f32)
        });

        // Create brightness slider
        let brightness = AppState::brightness(cx).get();
        let brightness_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(brightness.percentage() as f32)
        });

        // Subscribe to slider events
        let audio_services = AppState::audio(cx).clone();
        cx.subscribe(
            &volume_slider,
            move |_this, _slider, event: &SliderEvent, _cx| {
                let SliderEvent::Change(value) = event;
                let target = *value as u8;
                audio_services.dispatch(AudioCommand::SetSinkVolume(target));
            },
        )
        .detach();

        let brightness_services = AppState::brightness(cx).clone();
        cx.subscribe(
            &brightness_slider,
            move |_this, _slider, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event;
                let target = *value as u8;
                let s = brightness_services.clone();
                cx.spawn(async move |_, _| {
                    let _ = s.dispatch(BrightnessCommand::SetPercent(target)).await;
                })
                .detach();
            },
        )
        .detach();

        // Subscribe to service updates
        Self::subscribe_to_services(cx);

        ControlCenter {
            expanded: ExpandedSection::None,
            focus_handle,
            volume_slider,
            brightness_slider,
            wifi_password: WifiPasswordState::default(),
        }
    }

    /// Subscribe to service updates to keep UI in sync
    fn subscribe_to_services(cx: &mut Context<Self>) {
        // Audio - sync volume slider
        watch(
            cx,
            AppState::audio(cx).subscribe(),
            |control_center, data, cx| {
                let volume = data.sink_volume as f32;
                control_center.volume_slider.update(cx, |slider, cx| {
                    slider.set_value(volume, cx);
                });
                cx.notify();
            },
        );

        // Bluetooth
        watch(cx, AppState::bluetooth(cx).subscribe(), |_, _, cx| {
            cx.notify();
        });

        // Brightness - sync brightness slider
        watch(
            cx,
            AppState::brightness(cx).subscribe(),
            |control_center, data, cx| {
                let percent = data.percentage() as f32;
                control_center.brightness_slider.update(cx, |slider, cx| {
                    slider.set_value(percent, cx);
                });
                cx.notify();
            },
        );

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
        let on_toggle_section: Rc<dyn Fn(ExpandedSection, &mut App)> = Rc::new({
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
        let on_wifi_connect: Rc<dyn Fn(String, Option<String>, &mut App)> = Rc::new({
            let entity = entity.clone();
            let services = wifi_services.clone();
            move |ssid: String, password: Option<String>, cx: &mut App| {
                let entity = entity.clone();
                let services = services.clone();
                if let Some(pwd) = password {
                    // Connect with password
                    let network = services.get();
                    if let Some(ap) = network
                        .wireless_access_points
                        .iter()
                        .find(|a| a.ssid == ssid)
                    {
                        let ap_path: zbus::zvariant::OwnedObjectPath = ap.path.clone().into();
                        let device_path: zbus::zvariant::OwnedObjectPath =
                            ap.device_path.clone().into();
                        let password = if pwd.is_empty() { None } else { Some(pwd) };

                        entity.update(cx, |this, cx| {
                            this.wifi_password.connecting = true;
                            cx.notify();
                        });

                        cx.spawn({
                            let entity = entity.clone();
                            async move |cx| {
                                let result = services
                                    .dispatch(NetworkCommand::ConnectToAccessPoint {
                                        device_path,
                                        ap_path,
                                        password,
                                    })
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
                } else {
                    // Need password - prompt for one
                    entity.update(cx, |this, cx| {
                        this.wifi_password.start_for(ssid);
                        cx.notify();
                    });
                }
            }
        });

        let on_wifi_disconnect: Rc<dyn Fn(zbus::zvariant::OwnedObjectPath, &mut App)> = Rc::new({
            let services = wifi_services.clone();
            move |path: zbus::zvariant::OwnedObjectPath, cx: &mut App| {
                let s = services.clone();
                cx.spawn(async move |_| {
                    let _ = s.dispatch(NetworkCommand::Disconnect(path)).await;
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
            let gutter = spacing::SM * 2.0;
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
            let bg_secondary = theme.bg.secondary;
            let border_subtle = theme.border.subtle;
            let interactive_default = theme.interactive.default;
            let interactive_hover = theme.interactive.hover;
            let text_primary = theme.text.primary;
            let text_muted = theme.text.muted;
            let accent_primary = theme.accent.primary;

            let battery = upower.battery.as_ref();
            let battery_icon = battery.map(|b| b.icon()).unwrap_or(icons::BATTERY_FULL);
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
                    theme.status.error
                } else if b.is_charging() {
                    theme.status.success
                } else if b.percentage <= 20 {
                    theme.status.warning
                } else {
                    theme.text.primary
                }
            } else {
                theme.text.muted
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
                move |path: zbus::zvariant::OwnedObjectPath, cx: &mut App| {
                    (on_wifi_disconnect)(path, cx);
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
                .p(px(spacing::MD))
                .bg(theme.bg.primary)
                .border_1()
                .border_color(theme.border.default)
                .rounded(px(radius::LG))
                .text_color(theme.text.primary)
                .flex()
                .flex_col()
                .gap(px(spacing::MD))
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
                                let network = services.get();
                                if let Some(ap) = network
                                    .wireless_access_points
                                    .iter()
                                    .find(|a| a.ssid == ssid)
                                {
                                    let ap_path: zbus::zvariant::OwnedObjectPath =
                                        ap.path.clone().into();
                                    let device_path: zbus::zvariant::OwnedObjectPath =
                                        ap.device_path.clone().into();
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
                                                .dispatch(NetworkCommand::ConnectToAccessPoint {
                                                    device_path,
                                                    ap_path,
                                                    password,
                                                })
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
                        .gap(px(spacing::SM))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(spacing::SM))
                                .px(px(spacing::SM))
                                .py(px(spacing::XS))
                                .bg(bg_secondary)
                                .border_1()
                                .border_color(border_subtle)
                                .rounded(px(radius::MD))
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .gap(px(spacing::SM))
                                        .child(
                                            div()
                                                .text_size(px(icon_size::LG))
                                                .text_color(battery_color)
                                                .child(battery_icon),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(2.))
                                                .child(
                                                    div()
                                                        .text_size(px(font_size::SM))
                                                        .text_color(text_primary)
                                                        .child(battery_line),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(font_size::XS))
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
                                        .rounded(px(radius::MD))
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(interactive_hover))
                                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                            on_cycle_power_profile(cx);
                                        })
                                        .child(
                                            div()
                                                .text_size(px(icon_size::SM))
                                                .text_color(text_primary)
                                                .child(upower.power_profile.icon()),
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
                                    div()
                                        .text_size(px(icon_size::MD))
                                        .text_color(text_primary)
                                        .child(icons::POWER_BUTTON),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("control-center-volume")
                        .p(px(spacing::SM))
                        .bg(bg_secondary)
                        .border_1()
                        .border_color(border_subtle)
                        .rounded(px(radius::MD))
                        .child(sliders::render_volume_slider(&self.volume_slider, cx)),
                )
                .when(show_brightness, |el| {
                    el.child(
                        div()
                            .id("control-center-brightness")
                            .p(px(spacing::SM))
                            .bg(bg_secondary)
                            .border_1()
                            .border_color(border_subtle)
                            .rounded(px(radius::MD))
                            .child(sliders::render_brightness_slider(
                                &self.brightness_slider,
                                cx,
                            )),
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
                            .p(px(spacing::SM))
                            .bg(bg_secondary)
                            .border_1()
                            .border_color(border_subtle)
                            .rounded(px(radius::MD))
                            .flex()
                            .flex_col()
                            .gap(px(spacing::SM))
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
            measure_root.layout_as_root(available_space, window, cx).height
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
