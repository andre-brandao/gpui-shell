//! Quick toggle buttons for the Control Center.
//!
//! Provides compact modules for WiFi, Bluetooth, Microphone, and Camera status.

use gpui::{App, MouseButton, div, prelude::*, px};
use services::{AudioCommand, BluetoothCommand, BluetoothState, NetworkCommand};
use ui::{ActiveTheme, icon_size, radius, spacing};

use crate::state::AppState;

use super::icons;

/// Which section is currently expanded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpandedSection {
    #[default]
    None,
    WiFi,
    Bluetooth,
    Power,
}

/// Render the quick toggles row
pub fn render_quick_toggles(
    expanded: ExpandedSection,
    on_toggle_section: impl Fn(ExpandedSection, &mut App) + Clone + 'static,
    cx: &App,
) -> impl IntoElement {
    let network = AppState::network(cx).get();
    let bluetooth = AppState::bluetooth(cx).get();
    let audio = AppState::audio(cx).get();
    let privacy = AppState::privacy(cx).get();

    let wifi_enabled = network.wifi_enabled;
    let wifi_connected = network.active_connections.iter().any(|c| {
        matches!(c, services::ActiveConnectionInfo::WiFi { .. })
    });
    let wifi_name = network.active_connections.iter().find_map(|c| {
        if let services::ActiveConnectionInfo::WiFi { name, .. } = c {
            Some(name.clone())
        } else {
            None
        }
    });

    let bt_active = bluetooth.state == BluetoothState::Active;
    let bt_connected = bluetooth.devices.iter().filter(|d| d.connected).count();

    let mic_muted = audio.source_muted;
    let cam_active = privacy.webcam_access();

    let wifi_status = if !wifi_enabled {
        "Off".to_string()
    } else if let Some(name) = wifi_name.clone() {
        name
    } else if wifi_connected {
        "Connected".to_string()
    } else {
        "On".to_string()
    };

    let bt_status = if !bt_active {
        "Off".to_string()
    } else if bt_connected > 0 {
        format!("{} conn", bt_connected)
    } else {
        "On".to_string()
    };

    let mic_status = if mic_muted { "Muted" } else { "On" };
    let cam_status = if cam_active { "In use" } else { "Idle" };

    let services_wifi = AppState::network(cx).clone();
    let services_bt = AppState::bluetooth(cx).clone();
    let services_mic = AppState::audio(cx).clone();

    let on_toggle_wifi = on_toggle_section.clone();
    let on_toggle_bt = on_toggle_section.clone();

    div()
        .flex()
        .flex_col()
        .gap(px(spacing::SM))
        .w_full()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .w_full()
                .child(render_simple_module(
                    "mic-toggle",
                    if mic_muted {
                        icons::MICROPHONE_MUTE
                    } else {
                        icons::MICROPHONE
                    },
                    "Mic",
                    mic_status,
                    !mic_muted,
                    cx,
                    move |_cx| {
                        services_mic.dispatch(AudioCommand::ToggleSourceMute);
                    },
                ))
                .child(render_status_module(
                    "cam-status",
                    icons::CAMERA,
                    "Cam",
                    cam_status,
                    cam_active,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .w_full()
                .child(render_expandable_module(
                    "wifi-toggle",
                    if wifi_enabled { icons::WIFI } else { icons::WIFI_OFF },
                    "WiFi",
                    wifi_status,
                    wifi_enabled,
                    expanded == ExpandedSection::WiFi,
                    cx,
                    move |cx| {
                        let services = services_wifi.clone();
                        cx.spawn(async move |_| {
                            let _ = services.dispatch(NetworkCommand::ToggleWifi).await;
                        })
                        .detach();
                    },
                    move |cx| {
                        on_toggle_wifi(ExpandedSection::WiFi, cx);
                    },
                ))
                .child(render_expandable_module(
                    "bt-toggle",
                    if bt_active {
                        icons::BLUETOOTH
                    } else {
                        icons::BLUETOOTH_OFF
                    },
                    "Bluetooth",
                    bt_status,
                    bt_active,
                    expanded == ExpandedSection::Bluetooth,
                    cx,
                    move |cx| {
                        let services = services_bt.clone();
                        cx.spawn(async move |_| {
                            let _ = services.dispatch(BluetoothCommand::Toggle).await;
                        })
                        .detach();
                    },
                    move |cx| {
                        on_toggle_bt(ExpandedSection::Bluetooth, cx);
                    },
                )),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_expandable_module(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    status: String,
    active: bool,
    expanded: bool,
    cx: &App,
    on_toggle: impl Fn(&mut App) + 'static,
    on_expand: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    let bg_secondary = theme.bg.secondary;
    let border_subtle = theme.border.subtle;
    let interactive_hover = theme.interactive.hover;
    let accent_primary = theme.accent.primary;
    let text_primary = theme.text.primary;
    let text_secondary = theme.text.secondary;
    let text_muted = theme.text.muted;

    let border_color = if expanded {
        accent_primary
    } else {
        border_subtle
    };
    let icon_color = if active {
        accent_primary
    } else {
        text_muted
    };
    let status_color = if active {
        text_secondary
    } else {
        text_muted
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(spacing::XS))
        .flex_1()
        .rounded(px(radius::MD))
        .border_1()
        .border_color(border_color)
        .bg(bg_secondary)
        .overflow_hidden()
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .px(px(spacing::SM))
                .py(px(spacing::XS))
                .cursor_pointer()
                .hover(move |s| s.bg(interactive_hover))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_toggle(cx);
                })
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(icon_color)
                        .child(icon),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(theme.font_sizes.xs)
                                .text_color(text_primary)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(theme.font_sizes.xs)
                                .text_color(status_color)
                                .child(status),
                        ),
                ),
        )
        .child(
            div()
                .w(px(26.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(interactive_hover))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_expand(cx);
                })
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(text_muted)
                        .child(if expanded {
                            icons::CHEVRON_UP
                        } else {
                            icons::CHEVRON_DOWN
                        }),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_simple_module(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    status: &'static str,
    active: bool,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    let bg_secondary = theme.bg.secondary;
    let border_subtle = theme.border.subtle;
    let interactive_hover = theme.interactive.hover;
    let accent_primary = theme.accent.primary;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .flex_1()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .rounded(px(radius::MD))
        .border_1()
        .border_color(border_subtle)
        .bg(bg_secondary)
        .cursor_pointer()
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(if active {
                    accent_primary
                } else {
                    text_muted
                })
                .child(icon),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(theme.font_sizes.xs)
                        .text_color(text_primary)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(theme.font_sizes.xs)
                        .text_color(text_muted)
                        .child(status),
                ),
        )
}

fn render_status_module(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    status: &'static str,
    active: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();

    let bg_secondary = theme.bg.secondary;
    let border_subtle = theme.border.subtle;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let status_warning = theme.status.warning;

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .flex_1()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .rounded(px(radius::MD))
        .border_1()
        .border_color(border_subtle)
        .bg(bg_secondary)
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(if active {
                    status_warning
                } else {
                    text_muted
                })
                .child(icon),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(theme.font_sizes.xs)
                        .text_color(text_primary)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(theme.font_sizes.xs)
                        .text_color(text_muted)
                        .child(status),
                ),
        )
}
