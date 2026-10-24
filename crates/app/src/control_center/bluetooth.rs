//! Bluetooth section for the Control Center.
//!
//! Displays paired devices with connection status and battery levels.
//! Supports connecting and disconnecting from devices.

use gpui::{App, MouseButton, div, prelude::*, px};
use services::{BluetoothCommand, BluetoothDevice, BluetoothSubscriber};
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Spacing, Spinner, TextSize};

use crate::state::AppState;
use zbus::zvariant::OwnedObjectPath;

use super::list;
use super::tooltip::control_center_tooltip;
use crate::icons;

/// Render the Bluetooth section (device list)
pub fn render_bluetooth_section(cx: &App) -> impl IntoElement {
    let service = AppState::bluetooth(cx);
    let bluetooth = service.get();
    let pending = service.pending();
    let discovering = bluetooth.discovering;

    // Sort devices: connected first, then by name
    let mut devices: Vec<BluetoothDevice> = bluetooth.devices.clone();
    devices.sort_by(|a, b| {
        let rank = |d: &BluetoothDevice| {
            if d.connected {
                0
            } else if d.paired {
                1
            } else {
                2
            }
        };
        rank(a).cmp(&rank(b)).then_with(|| a.name.cmp(&b.name))
    });

    list::section(
        IconName::Bluetooth,
        "Bluetooth",
        |header| header.child(render_scan_button(discovering, cx)),
        cx,
    )
    .when(discovering, |el| el.child(list::hint("Scanning...", cx)))
    .when(devices.is_empty(), |el| {
        el.child(list::empty_state("No paired devices", cx))
    })
    .when(!devices.is_empty(), |el| {
        el.child(list::list("bluetooth-devices-list", cx).children(
            devices.into_iter().enumerate().map(|(idx, device)| {
                let activity = device_activity(&pending, &device.path);
                render_device_item(idx, device, activity, service, cx)
            }),
        ))
    })
}

/// What the queue is doing to a device right now, as a row label.
fn device_activity(pending: &[BluetoothCommand], path: &OwnedObjectPath) -> Option<&'static str> {
    pending.iter().rev().find_map(|command| match command {
        BluetoothCommand::ConnectDevice(p) if p == path => Some("Connecting..."),
        BluetoothCommand::DisconnectDevice(p) if p == path => Some("Disconnecting..."),
        BluetoothCommand::PairDevice(p) if p == path => Some("Pairing..."),
        BluetoothCommand::RemoveDevice(p) if p == path => Some("Removing..."),
        _ => None,
    })
}

/// Render one device row. While `activity` is set the row shows it with a
/// spinner in place of the actions, and ignores clicks.
fn render_device_item(
    index: usize,
    device: BluetoothDevice,
    activity: Option<&'static str>,
    service: &BluetoothSubscriber,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let connected = device.connected;
    let paired = device.paired;
    let path = device.path.clone();
    let text_muted = theme.colors.text_muted;
    let status_success = theme.colors.status.success;

    // Each action button dispatches one command for this device.
    let action =
        |id: &str, icon, color, tooltip, command: fn(OwnedObjectPath) -> BluetoothCommand| {
            let service = service.clone();
            let path = path.clone();
            list::action_button(
                format!("bt-{id}-{index}"),
                icon,
                color,
                tooltip,
                move |_| service.dispatch(command(path.clone())),
                cx,
            )
        };

    list::row(format!("bt-device-{index}"), connected, cx)
        .on_mouse_down(MouseButton::Left, {
            let service = service.clone();
            let path = path.clone();
            move |_, _, _| {
                if connected || activity.is_some() {
                    return;
                }
                service.dispatch(if paired {
                    BluetoothCommand::ConnectDevice(path.clone())
                } else {
                    BluetoothCommand::PairDevice(path.clone())
                });
            }
        })
        .child(list::row_icon(
            format!("bt-device-icon-{index}"),
            get_device_icon(&device),
            connected,
            get_device_icon_tooltip(&device),
            cx,
        ))
        .child(list::row_label(device.name.clone(), cx))
        .when(paired && !connected, |el| {
            el.child(list::badge(
                format!("bt-paired-{index}"),
                IconName::Check,
                status_success,
                "Paired",
            ))
        })
        // Battery level (if available)
        .when_some(device.battery, |el, level| {
            el.child(render_battery_indicator(index, level, cx))
        })
        .when_some(activity, |el, label| {
            el.child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::XSmall.pixels())
                    .text_size(TextSize::XSmall.rems())
                    .text_color(text_muted)
                    .child(label)
                    .child(Spinner::new().size(IconSize::XSmall)),
            )
        })
        .when(activity.is_none(), |el| {
            el.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .when(!paired, |el| {
                        el.child(action(
                            "pair",
                            IconName::Plus,
                            text_muted,
                            "Pair",
                            BluetoothCommand::PairDevice,
                        ))
                    })
                    .when(paired && !connected, |el| {
                        el.child(action(
                            "connect",
                            IconName::ChevronRight,
                            text_muted,
                            "Connect",
                            BluetoothCommand::ConnectDevice,
                        ))
                    })
                    .when(connected, |el| {
                        el.child(action(
                            "disconnect",
                            IconName::Close,
                            status_success,
                            "Disconnect",
                            BluetoothCommand::DisconnectDevice,
                        ))
                    })
                    .when(paired, |el| {
                        el.child(action(
                            "remove",
                            IconName::Trash,
                            text_muted,
                            "Remove device",
                            BluetoothCommand::RemoveDevice,
                        ))
                    }),
            )
        })
}

/// Render battery indicator for a device
fn render_battery_indicator(index: usize, level: u8, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let color = if level <= 20 {
        theme.colors.status.error
    } else if level <= 40 {
        theme.colors.status.warning
    } else {
        theme.colors.text_muted
    };

    let icon = icons::battery_icon(level, false);

    div()
        .id(format!("bt-battery-{}", index))
        .flex()
        .items_center()
        .gap(px(2.))
        .tooltip(control_center_tooltip(format!("Battery: {}%", level)))
        .child(
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(color)),
        )
        .child(
            div()
                .text_size(TextSize::XSmall.rems())
                .text_color(color)
                .child(format!("{}%", level)),
        )
}

/// Get appropriate icon for device type
fn get_device_icon(device: &BluetoothDevice) -> IconName {
    // Check device class/type from icon or name hints
    let name_lower = device.name.to_lowercase();

    if name_lower.contains("airpod")
        || name_lower.contains("headphone")
        || name_lower.contains("buds")
    {
        IconName::Headphones
    } else if name_lower.contains("mouse") {
        IconName::Mouse
    } else if name_lower.contains("keyboard") {
        IconName::Keyboard
    } else if name_lower.contains("speaker") || name_lower.contains("soundbar") {
        IconName::Volume
    } else if name_lower.contains("phone")
        || name_lower.contains("iphone")
        || name_lower.contains("android")
    {
        IconName::Phone
    } else if name_lower.contains("watch") {
        IconName::Clock
    } else if name_lower.contains("controller") || name_lower.contains("gamepad") {
        IconName::Gamepad
    } else if device.connected {
        IconName::BluetoothConnected
    } else {
        IconName::Bluetooth
    }
}

/// Get tooltip label for device icon based on device type.
fn get_device_icon_tooltip(device: &BluetoothDevice) -> &'static str {
    let name_lower = device.name.to_lowercase();

    if name_lower.contains("airpod")
        || name_lower.contains("headphone")
        || name_lower.contains("buds")
    {
        "Headphones"
    } else if name_lower.contains("mouse") {
        "Mouse"
    } else if name_lower.contains("keyboard") {
        "Keyboard"
    } else if name_lower.contains("speaker") || name_lower.contains("soundbar") {
        "Speaker"
    } else if name_lower.contains("phone")
        || name_lower.contains("iphone")
        || name_lower.contains("android")
    {
        "Phone"
    } else if name_lower.contains("watch") {
        "Watch"
    } else if name_lower.contains("controller") || name_lower.contains("gamepad") {
        "Gamepad"
    } else {
        "Bluetooth device"
    }
}

/// Render scan button for discovering devices
fn render_scan_button(discovering: bool, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let services = AppState::bluetooth(cx).clone();

    let (bg_color, icon_color) = if discovering {
        (theme.colors.accent, theme.colors.background)
    } else {
        (theme.colors.element_background, theme.colors.text_muted)
    };
    let hover_color = if discovering {
        theme.colors.accent
    } else {
        theme.colors.element_hover
    };

    list::header_button("bt-scan", cx)
        .cursor_pointer()
        .bg(bg_color)
        .hover(move |s| s.bg(hover_color))
        .on_mouse_down(MouseButton::Left, move |_, _, _| {
            services.dispatch(if discovering {
                BluetoothCommand::StopDiscovery
            } else {
                BluetoothCommand::StartDiscovery
            });
        })
        .child(
            Icon::new(IconName::Refresh)
                .size(IconSize::XSmall)
                .color(Color::Custom(icon_color)),
        )
}
