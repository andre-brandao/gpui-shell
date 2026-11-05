//! Quick toggle buttons for the Control Center.
//!
//! Provides compact modules for WiFi, Bluetooth, VPN, Microphone, and Camera
//! status.

use gpui::{App, Hsla, MouseButton, div, prelude::*, px};
use services::{AudioCommand, BluetoothCommand, BluetoothState, NetworkCommand};
use ui::patterns::surface_radius;
use ui::{ActiveTheme, Color, Fade, Icon, IconName, IconSize, Opacity, Spacing, TextSize};

use crate::state::AppState;

/// Which section is currently expanded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpandedSection {
    #[default]
    None,
    WiFi,
    Bluetooth,
    Vpn,
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
    let wifi_connected = network
        .active_connections
        .iter()
        .any(|c| matches!(c, services::ActiveConnectionInfo::WiFi { .. }));
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

    // Shown only when there is a profile to toggle.
    let vpns = network.vpn_profiles.clone();
    let active_vpns: Vec<_> = vpns.iter().filter(|v| v.active).cloned().collect();
    let vpn_status = match active_vpns.as_slice() {
        [] => "Off".to_string(),
        [vpn] => vpn.name.clone(),
        many => format!("{} on", many.len()),
    };

    let mic_status = if mic_muted { "Muted" } else { "On" };
    let cam_status = if cam_active { "In use" } else { "Idle" };

    let services_wifi = AppState::network(cx).clone();
    let services_bt = AppState::bluetooth(cx).clone();
    let services_mic = AppState::audio(cx).clone();

    let on_toggle_wifi = on_toggle_section.clone();
    let on_toggle_bt = on_toggle_section.clone();
    let on_toggle_vpn = on_toggle_section.clone();
    let on_expand_vpn = on_toggle_section.clone();

    div()
        .flex()
        .flex_col()
        .gap(Spacing::Medium.pixels())
        .w_full()
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .w_full()
                .child(render_simple_module(
                    "mic-toggle",
                    if mic_muted {
                        IconName::MicOff
                    } else {
                        IconName::Mic
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
                    IconName::Camera,
                    "Cam",
                    cam_status,
                    cam_active,
                    cx,
                ))
                .when(!vpns.is_empty(), |el| {
                    let services_vpn = services_wifi.clone();
                    el.child(render_expandable_module(
                        "vpn-toggle",
                        IconName::Shield,
                        "VPN",
                        vpn_status,
                        !active_vpns.is_empty(),
                        expanded == ExpandedSection::Vpn,
                        cx,
                        // Off when any is on; otherwise pick one from the list.
                        move |cx| {
                            if active_vpns.is_empty() {
                                on_toggle_vpn(ExpandedSection::Vpn, cx);
                            }
                            for vpn in &active_vpns {
                                services_vpn
                                    .dispatch(NetworkCommand::DisconnectVpn(vpn.path.clone()));
                            }
                        },
                        move |cx| on_expand_vpn(ExpandedSection::Vpn, cx),
                    ))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .w_full()
                .child(render_expandable_module(
                    "wifi-toggle",
                    if wifi_enabled {
                        IconName::Wifi
                    } else {
                        IconName::WifiOff
                    },
                    "WiFi",
                    wifi_status,
                    wifi_enabled,
                    expanded == ExpandedSection::WiFi,
                    cx,
                    move |_| services_wifi.dispatch(NetworkCommand::ToggleWifi),
                    move |cx| {
                        on_toggle_wifi(ExpandedSection::WiFi, cx);
                    },
                ))
                .child(render_expandable_module(
                    "bt-toggle",
                    if bt_active {
                        IconName::Bluetooth
                    } else {
                        IconName::BluetoothOff
                    },
                    "Bluetooth",
                    bt_status,
                    bt_active,
                    expanded == ExpandedSection::Bluetooth,
                    cx,
                    move |_| services_bt.dispatch(BluetoothCommand::Toggle),
                    move |cx| {
                        on_toggle_bt(ExpandedSection::Bluetooth, cx);
                    },
                )),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_expandable_module(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    status: String,
    active: bool,
    expanded: bool,
    cx: &App,
    on_toggle: impl Fn(&mut App) + 'static,
    on_expand: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    let ink = ToggleInk::new(theme, active);
    // Expanded shows as an accent outline on an off toggle; an on toggle is
    // already accent all over.
    let border = if expanded && !active {
        theme.colors.accent
    } else {
        ink.border
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(Spacing::XSmall.pixels())
        .flex_1()
        .rounded(surface_radius())
        .border_1()
        .border_color(border)
        .bg(ink.background)
        .overflow_hidden()
        .child(
            // gpui repaints a hover change at once only on elements with an id.
            div()
                .id("toggle")
                .flex_1()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .px(Spacing::Medium.pixels())
                .py(Spacing::XSmall.pixels())
                .cursor_pointer()
                .hover(move |s| s.bg(ink.hover))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_toggle(cx);
                })
                .child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .color(Color::Custom(ink.icon)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(Spacing::XXSmall.pixels())
                        .child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(ink.text)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(ink.muted)
                                .child(status),
                        ),
                ),
        )
        .child(
            div()
                .id("expand")
                .w(px(26.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(ink.hover))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_expand(cx);
                })
                .child(
                    Icon::new(if expanded {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .size(IconSize::XSmall)
                    .color(Color::Custom(ink.muted)),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
fn render_simple_module(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    status: &'static str,
    active: bool,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    // One selection language: an active toggle is an accent fill with the
    // text on it, like an active workspace.
    let ink = ToggleInk::new(theme, active);

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .flex_1()
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
        .rounded(surface_radius())
        .border_1()
        .border_color(ink.border)
        .bg(ink.background)
        .cursor_pointer()
        .hover(move |s| s.bg(ink.hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(ink.icon)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::XXSmall.pixels())
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
                        .text_color(ink.text)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
                        .text_color(ink.muted)
                        .child(status),
                ),
        )
}

/// Colours of a toggle card in one of its two states: accent-filled when
/// on, a plain card when off.
#[derive(Clone, Copy)]
struct ToggleInk {
    background: Hsla,
    hover: Hsla,
    border: Hsla,
    icon: Hsla,
    text: Hsla,
    muted: Hsla,
}

impl ToggleInk {
    fn new(theme: &ui::Theme, active: bool) -> Self {
        let colors = theme.colors();
        if active {
            Self {
                background: colors.accent,
                hover: colors.accent.fade(Opacity::Soft),
                border: colors.accent,
                icon: colors.background,
                text: colors.background,
                muted: colors.background.fade(Opacity::Soft),
            }
        } else {
            Self {
                background: colors.surface_background,
                hover: colors.element_hover,
                border: colors.border_variant,
                icon: colors.text_muted,
                text: colors.text,
                muted: colors.text_muted,
            }
        }
    }
}

fn render_status_module(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    status: &'static str,
    active: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();

    let bg_secondary = theme.colors.surface_background;
    let border_subtle = theme.colors.border_variant;
    let text_primary = theme.colors.text;
    let text_muted = theme.colors.text_muted;
    let status_warning = theme.colors.status.warning;

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .flex_1()
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
        .rounded(surface_radius())
        .border_1()
        .border_color(border_subtle)
        .bg(bg_secondary)
        .child(
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(if active {
                    status_warning
                } else {
                    text_muted
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
                        .text_color(text_primary)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
                        .text_color(text_muted)
                        .child(status),
                ),
        )
}
