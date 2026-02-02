//! Quick toggle buttons for the Control Center.
//!
//! Provides compact toggle buttons for WiFi, Bluetooth, and Microphone.

use gpui::{App, MouseButton, div, prelude::*, px};
use services::{AudioCommand, BluetoothCommand, BluetoothState, NetworkCommand, Services};
use ui::{accent, icon_size, interactive, radius, spacing, text};

use super::icons;

/// Which section is currently expanded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpandedSection {
    #[default]
    None,
    WiFi,
    Bluetooth,
}

/// Render the quick toggles row
pub fn render_quick_toggles(
    services: &Services,
    expanded: ExpandedSection,
    on_toggle_section: impl Fn(ExpandedSection, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let network = services.network.get();
    let bluetooth = services.bluetooth.get();
    let audio = services.audio.get();

    let wifi_enabled = network.wifi_enabled;
    let bt_active = bluetooth.state == BluetoothState::Active;
    let mic_muted = audio.source_muted;

    let services_wifi = services.clone();
    let services_bt = services.clone();
    let services_mic = services.clone();

    let on_toggle_wifi = on_toggle_section.clone();
    let on_toggle_bt = on_toggle_section.clone();

    div()
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .w_full()
        // WiFi toggle
        .child(render_expandable_toggle(
            "wifi-toggle",
            if wifi_enabled {
                icons::WIFI
            } else {
                icons::WIFI_OFF
            },
            wifi_enabled,
            expanded == ExpandedSection::WiFi,
            move |cx| {
                let services = services_wifi.clone();
                cx.spawn(async move |_| {
                    let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
                })
                .detach();
            },
            move |cx| {
                on_toggle_wifi(ExpandedSection::WiFi, cx);
            },
        ))
        // Bluetooth toggle
        .child(render_expandable_toggle(
            "bt-toggle",
            if bt_active {
                icons::BLUETOOTH
            } else {
                icons::BLUETOOTH_OFF
            },
            bt_active,
            expanded == ExpandedSection::Bluetooth,
            move |cx| {
                let services = services_bt.clone();
                cx.spawn(async move |_| {
                    let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
                })
                .detach();
            },
            move |cx| {
                on_toggle_bt(ExpandedSection::Bluetooth, cx);
            },
        ))
        // Microphone toggle (simple, no expand)
        .child(render_simple_toggle(
            "mic-toggle",
            if mic_muted {
                icons::MICROPHONE_MUTE
            } else {
                icons::MICROPHONE
            },
            !mic_muted,
            move |_cx| {
                services_mic.audio.dispatch(AudioCommand::ToggleSourceMute);
            },
        ))
}

/// Render an expandable toggle button (left click = toggle, right click = expand)
fn render_expandable_toggle(
    id: &'static str,
    icon: &'static str,
    active: bool,
    expanded: bool,
    on_toggle: impl Fn(&mut App) + 'static,
    on_expand: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(2.))
        .rounded(px(radius::MD))
        .overflow_hidden()
        .child(
            // Main toggle button
            div()
                .id(format!("{}-main", id))
                .flex()
                .items_center()
                .justify_center()
                .w(px(40.))
                .h(px(36.))
                .cursor_pointer()
                .when(active, |el| el.bg(accent::primary()))
                .when(!active, |el| el.bg(interactive::default()))
                .hover(|s| s.bg(interactive::hover()))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_toggle(cx);
                })
                .child(
                    div()
                        .text_size(px(icon_size::MD))
                        .text_color(text::primary())
                        .child(icon),
                ),
        )
        .child(
            // Expand button
            div()
                .id(format!("{}-expand", id))
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.))
                .h(px(36.))
                .cursor_pointer()
                .when(expanded, |el| el.bg(accent::selection()))
                .when(!expanded, |el| el.bg(interactive::default()))
                .hover(|s| s.bg(interactive::hover()))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    on_expand(cx);
                })
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(text::muted())
                        .child(if expanded {
                            icons::CHEVRON_UP
                        } else {
                            icons::CHEVRON_DOWN
                        }),
                ),
        )
}

/// Render a simple toggle button (no expand functionality)
fn render_simple_toggle(
    id: &'static str,
    icon: &'static str,
    active: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(40.))
        .h(px(36.))
        .rounded(px(radius::MD))
        .cursor_pointer()
        .when(active, |el| el.bg(accent::primary()))
        .when(!active, |el| el.bg(interactive::default()))
        .hover(|s| s.bg(interactive::hover()))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(icon_size::MD))
                .text_color(text::primary())
                .child(icon),
        )
}
