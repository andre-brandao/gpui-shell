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
pub mod icons;
mod power;
mod quick_toggles;
mod sliders;
mod wifi;

use futures_signals::signal::SignalExt;
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, KeyBinding, ScrollHandle, Window, actions, div,
    prelude::*, px,
};
use services::{AudioCommand, BrightnessCommand, NetworkCommand, Services};
use ui::{ActiveTheme, Slider, SliderEvent, radius, spacing};

// Keyboard actions for password input
actions!(control_center, [Backspace, Cancel, Submit, TypeChar]);

pub use quick_toggles::ExpandedSection;
pub use wifi::WifiPasswordState;

/// Control Center panel component.
///
/// Provides a unified interface for system settings and quick actions.
pub struct ControlCenter {
    /// Services container for system interactions
    services: Services,
    /// Currently expanded section (WiFi or Bluetooth)
    expanded: ExpandedSection,
    /// Scroll handle for the panel content
    scroll_handle: ScrollHandle,
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
    /// Register keybindings for the control center
    pub fn register_keybindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, Some("ControlCenter")),
            KeyBinding::new("escape", Cancel, Some("ControlCenter")),
            KeyBinding::new("enter", Submit, Some("ControlCenter")),
        ]);
    }

    /// Create a new control center panel.
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Create volume slider
        let audio = services.audio.get();
        let volume_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(audio.sink_volume as f32)
        });

        // Create brightness slider
        let brightness = services.brightness.get();
        let brightness_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(brightness.percentage() as f32)
        });

        // Subscribe to slider events
        let audio_services = services.clone();
        cx.subscribe(
            &volume_slider,
            move |_this, _slider, event: &SliderEvent, _cx| {
                let SliderEvent::Change(value) = event;
                let target = *value as u8;
                audio_services
                    .audio
                    .dispatch(AudioCommand::SetSinkVolume(target));
            },
        )
        .detach();

        let brightness_services = services.clone();
        cx.subscribe(
            &brightness_slider,
            move |_this, _slider, event: &SliderEvent, cx| {
                let SliderEvent::Change(value) = event;
                let target = *value as u8;
                let s = brightness_services.clone();
                cx.spawn(async move |_, _| {
                    let _ = s
                        .brightness
                        .dispatch(BrightnessCommand::SetPercent(target))
                        .await;
                })
                .detach();
            },
        )
        .detach();

        // Subscribe to service updates
        Self::subscribe_to_services(&services, cx);

        ControlCenter {
            services,
            expanded: ExpandedSection::None,
            scroll_handle,
            focus_handle,
            volume_slider,
            brightness_slider,
            wifi_password: WifiPasswordState::default(),
        }
    }

    /// Subscribe to service updates to keep UI in sync
    fn subscribe_to_services(services: &Services, cx: &mut Context<Self>) {
        // Audio - sync volume slider
        cx.spawn({
            let mut signal = services.audio.subscribe().to_stream();
            let audio_service = services.audio.clone();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    let audio = audio_service.get();
                    let volume = audio.sink_volume as f32;
                    if this
                        .update(cx, |control_center, cx| {
                            control_center.volume_slider.update(cx, |slider, cx| {
                                slider.set_value(volume, cx);
                            });
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

        // Bluetooth
        cx.spawn({
            let mut signal = services.bluetooth.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // Brightness - sync brightness slider
        cx.spawn({
            let mut signal = services.brightness.subscribe().to_stream();
            let brightness_service = services.brightness.clone();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    let brightness = brightness_service.get();
                    let percent = brightness.percentage() as f32;
                    if this
                        .update(cx, |control_center, cx| {
                            control_center.brightness_slider.update(cx, |slider, cx| {
                                slider.set_value(percent, cx);
                            });
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

        // Network
        cx.spawn({
            let mut signal = services.network.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        // UPower
        cx.spawn({
            let mut signal = services.upower.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        })
        .detach();
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let expanded = self.expanded;
        let services = self.services.clone();

        // Create entity-based callbacks for section toggling
        let entity = cx.entity().clone();
        let on_toggle_section = {
            let entity = entity.clone();
            move |section: ExpandedSection, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.toggle_section(section);
                    cx.notify();
                });
            }
        };

        // WiFi callbacks
        let wifi_services = services.clone();
        let on_wifi_connect = {
            let entity = entity.clone();
            let services = wifi_services.clone();
            move |ssid: String, password: Option<String>, cx: &mut App| {
                let entity = entity.clone();
                let services = services.clone();
                if let Some(pwd) = password {
                    // Connect with password
                    let network = services.network.get();
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
                                    .network
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
        };

        let on_password_change = {
            let entity = entity.clone();
            move |password: String, cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.wifi_password.password = password;
                    cx.notify();
                });
            }
        };

        let on_cancel_password = {
            let entity = entity.clone();
            move |cx: &mut App| {
                entity.update(cx, |this, cx| {
                    this.wifi_password.clear();
                    cx.notify();
                });
            }
        };

        div()
            .id("control-center")
            .track_focus(&self.focus_handle)
            .key_context("ControlCenter")
            .w_full()
            .h_full()
            .p(px(spacing::MD))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(radius::LG))
            .text_color(theme.text.primary)
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            // Keyboard event handling for password input
            .on_action({
                let entity = entity.clone();
                move |_: &Backspace, _window, cx| {
                    entity.update(cx, |this, cx| {
                        if this.wifi_password.ssid.is_some() {
                            this.wifi_password.password.pop();
                            cx.notify();
                        }
                    });
                }
            })
            .on_action({
                let entity = entity.clone();
                move |_: &Cancel, _window, cx| {
                    entity.update(cx, |this, cx| {
                        this.wifi_password.clear();
                        cx.notify();
                    });
                }
            })
            .on_action({
                let entity = entity.clone();
                let services = services.clone();
                move |_: &Submit, _window, cx| {
                    let entity = entity.clone();
                    let services = services.clone();
                    entity.update(cx, |this, cx| {
                        if let Some(ssid) = this.wifi_password.ssid.clone() {
                            let password = this.wifi_password.password.clone();
                            let network = services.network.get();
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
                                            .network
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
                    // Handle printable character input for password
                    let key = event.keystroke.key.as_str();
                    if key.len() == 1 && !event.keystroke.modifiers.control {
                        let ch = key.chars().next().unwrap();
                        if ch.is_ascii_graphic() || ch == ' ' {
                            entity.update(cx, |this, cx| {
                                if this.wifi_password.ssid.is_some() {
                                    this.wifi_password.password.push(ch);
                                    cx.notify();
                                }
                            });
                        }
                    }
                }
            })
            // Quick toggles row
            .child(quick_toggles::render_quick_toggles(
                &self.services,
                expanded,
                on_toggle_section,
                cx,
            ))
            // WiFi section (when expanded) - right after toggle
            .when(expanded == ExpandedSection::WiFi, |el| {
                el.child(wifi::render_wifi_section(
                    &services,
                    &self.wifi_password,
                    on_wifi_connect,
                    on_password_change,
                    on_cancel_password,
                    cx,
                ))
            })
            // Bluetooth section (when expanded) - right after toggle
            .when(expanded == ExpandedSection::Bluetooth, |el| {
                el.child(bluetooth::render_bluetooth_section(&self.services, cx))
            })
            // Power section (when expanded) - right after toggle
            .when(expanded == ExpandedSection::Power, |el| {
                el.child(power::render_power_section(&self.services, cx))
            })
            // Volume slider
            .child(sliders::render_volume_slider(
                &self.services,
                &self.volume_slider,
                cx,
            ))
            // Brightness slider (if available)
            .child(sliders::render_brightness_slider(
                &self.services,
                &self.brightness_slider,
                cx,
            ))
    }
}
