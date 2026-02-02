//! Bluetooth section for the Control Center.
//!
//! Displays paired devices with connection status and battery levels.
//! Supports connecting and disconnecting from devices.

use gpui::{App, ElementId, MouseButton, SharedString, div, prelude::*, px};
use services::{BluetoothCommand, BluetoothDevice, Services};
use ui::{accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text};
use zbus::zvariant::OwnedObjectPath;

use super::icons;

/// Render the Bluetooth section (device list)
pub fn render_bluetooth_section(services: &Services) -> impl IntoElement {
    let bluetooth = services.bluetooth.get();
    let services_clone = services.clone();

    // Sort devices: connected first, then by name
    let mut devices: Vec<BluetoothDevice> = bluetooth.devices.clone();
    devices.sort_by(|a, b| match (a.connected, b.connected) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::XS))
        .p(px(spacing::SM))
        .bg(bg::secondary())
        .rounded(px(radius::MD))
        .border_1()
        .border_color(border::subtle())
        .child(
            // Section header
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb(px(spacing::XS))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(spacing::SM))
                        .child(
                            div()
                                .text_size(px(icon_size::SM))
                                .text_color(text::muted())
                                .child(icons::BLUETOOTH),
                        )
                        .child(
                            div()
                                .text_size(px(font_size::SM))
                                .text_color(text::secondary())
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Devices"),
                        ),
                )
                .child(render_scan_button(&services_clone)),
        )
        .when(devices.is_empty(), |el| {
            el.child(
                div()
                    .py(px(spacing::MD))
                    .text_size(px(font_size::SM))
                    .text_color(text::muted())
                    .text_center()
                    .child("No paired devices"),
            )
        })
        .when(!devices.is_empty(), |el| {
            el.child(
                div()
                    .id("bluetooth-devices-list")
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .max_h(px(200.))
                    .overflow_y_scroll()
                    .children(devices.into_iter().enumerate().map(|(idx, device)| {
                        let services = services_clone.clone();
                        render_device_item(idx, device, move |path, connected, cx| {
                            let s = services.clone();
                            cx.spawn(async move |_| {
                                if connected {
                                    let _ = s
                                        .bluetooth
                                        .dispatch(BluetoothCommand::DisconnectDevice(path))
                                        .await;
                                } else {
                                    let _ = s
                                        .bluetooth
                                        .dispatch(BluetoothCommand::ConnectDevice(path))
                                        .await;
                                }
                            })
                            .detach();
                        })
                    })),
            )
        })
}

/// Render a single device item in the list
fn render_device_item(
    index: usize,
    device: BluetoothDevice,
    on_click: impl Fn(OwnedObjectPath, bool, &mut App) + 'static,
) -> impl IntoElement {
    let name = device.name.clone();
    let connected = device.connected;
    let battery = device.battery;
    let path = device.path.clone();
    let device_icon = get_device_icon(&device);

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "bt-device-{}",
            index
        ))))
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .w_full()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .when(connected, |el| el.bg(accent::selection()))
        .when(!connected, |el| el.hover(|s| s.bg(interactive::hover())))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(path.clone(), connected, cx);
        })
        // Device icon
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(if connected {
                    accent::primary()
                } else {
                    text::muted()
                })
                .child(device_icon),
        )
        // Device name
        .child(
            div()
                .flex_1()
                .text_size(px(font_size::SM))
                .text_color(text::primary())
                .overflow_hidden()
                .child(name),
        )
        // Battery level (if available)
        .when_some(battery, |el, level| {
            el.child(render_battery_indicator(level))
        })
        // Connection status
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(if connected {
                    status::success()
                } else {
                    text::muted()
                })
                .child(if connected { "Connected" } else { "Paired" }),
        )
}

/// Render battery indicator for a device
fn render_battery_indicator(level: u8) -> impl IntoElement {
    let color = if level <= 20 {
        status::error()
    } else if level <= 40 {
        status::warning()
    } else {
        text::muted()
    };

    let icon = icons::battery_icon(level, false);

    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(color)
                .child(icon),
        )
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(color)
                .child(format!("{}%", level)),
        )
}

/// Get appropriate icon for device type
fn get_device_icon(device: &BluetoothDevice) -> &'static str {
    // Check device class/type from icon or name hints
    let name_lower = device.name.to_lowercase();

    if name_lower.contains("airpod")
        || name_lower.contains("headphone")
        || name_lower.contains("buds")
    {
        "󰋋" // Headphones
    } else if name_lower.contains("mouse") {
        "󰍽" // Mouse
    } else if name_lower.contains("keyboard") {
        "󰌌" // Keyboard
    } else if name_lower.contains("speaker") || name_lower.contains("soundbar") {
        "󰓃" // Speaker
    } else if name_lower.contains("phone")
        || name_lower.contains("iphone")
        || name_lower.contains("android")
    {
        "󰏲" // Phone
    } else if name_lower.contains("watch") {
        "󰖉" // Watch
    } else if name_lower.contains("controller") || name_lower.contains("gamepad") {
        "󰊴" // Gamepad
    } else if device.connected {
        icons::BLUETOOTH_CONNECTED
    } else {
        icons::BLUETOOTH
    }
}

/// Render scan button for discovering devices
fn render_scan_button(services: &Services) -> impl IntoElement {
    let services = services.clone();

    div()
        .id("bt-scan")
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .bg(interactive::default())
        .hover(|s| s.bg(interactive::hover()))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            let s = services.clone();
            cx.spawn(async move |_| {
                let _ = s.bluetooth.dispatch(BluetoothCommand::StartDiscovery).await;
            })
            .detach();
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(text::muted())
                .child(icons::REFRESH),
        )
}
