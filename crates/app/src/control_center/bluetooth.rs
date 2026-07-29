//! Bluetooth section for the Control Center.
//!
//! Displays paired devices with connection status and battery levels.
//! Supports connecting and disconnecting from devices.

use gpui::{App, ElementId, MouseButton, SharedString, div, prelude::*, px};
use services::{BluetoothCommand, BluetoothDevice};
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use crate::state::AppState;
use zbus::zvariant::OwnedObjectPath;

use super::tooltip::control_center_tooltip;
use crate::icons;

/// Render the Bluetooth section (device list)
pub fn render_bluetooth_section(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let bluetooth = AppState::bluetooth(cx).get();
    let services_clone = AppState::bluetooth(cx).clone();
    let discovering = bluetooth.discovering;
    let list_bg = theme.colors.background;
    let list_border = theme.colors.border_variant;

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

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(Spacing::Medium.pixels())
        .child(
            // Section header
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(Spacing::Medium.pixels())
                        .child(
                            Icon::new(icons::BLUETOOTH)
                                .size(IconSize::XSmall)
                                .color(Color::Custom(theme.colors.text)),
                        )
                        .child(
                            div()
                                .text_size(TextSize::Small.rems())
                                .text_color(theme.colors.text)
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Bluetooth"),
                        ),
                )
                .child(render_scan_button(discovering, cx)),
        )
        .when(discovering, |el| {
            el.child(
                div()
                    .text_size(TextSize::XSmall.rems())
                    .text_color(theme.colors.text_muted)
                    .child("Scanning..."),
            )
        })
        .when(devices.is_empty(), |el| {
            el.child(
                div()
                    .py(Spacing::Large.pixels())
                    .text_size(TextSize::Small.rems())
                    .text_color(theme.colors.text_muted)
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
                    .max_h(px(240.))
                    .overflow_y_scroll()
                    .bg(list_bg)
                    .border_1()
                    .border_color(list_border)
                    .rounded(Radius::Small.pixels())
                    .py(Spacing::XSmall.pixels())
                    .children(devices.into_iter().enumerate().map(|(idx, device)| {
                        let services_connect = services_clone.clone();
                        let services_disconnect = services_clone.clone();
                        let services_pair = services_clone.clone();
                        let services_remove = services_clone.clone();

                        render_device_item(
                            idx,
                            device,
                            cx,
                            move |path, cx| {
                                let s = services_connect.clone();
                                cx.spawn(async move |_| {
                                    let _ = s.dispatch(BluetoothCommand::ConnectDevice(path)).await;
                                })
                                .detach();
                            },
                            move |path, cx| {
                                let s = services_disconnect.clone();
                                cx.spawn(async move |_| {
                                    let _ =
                                        s.dispatch(BluetoothCommand::DisconnectDevice(path)).await;
                                })
                                .detach();
                            },
                            move |path, cx| {
                                let s = services_pair.clone();
                                cx.spawn(async move |_| {
                                    let _ = s.dispatch(BluetoothCommand::PairDevice(path)).await;
                                })
                                .detach();
                            },
                            move |path, cx| {
                                let s = services_remove.clone();
                                cx.spawn(async move |_| {
                                    let _ = s.dispatch(BluetoothCommand::RemoveDevice(path)).await;
                                })
                                .detach();
                            },
                        )
                    })),
            )
        })
}

/// Render a single device item in the list
fn render_device_item(
    index: usize,
    device: BluetoothDevice,
    cx: &App,
    on_connect: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_disconnect: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_pair: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_remove: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    let name = device.name.clone();
    let connected = device.connected;
    let battery = device.battery;
    let path = device.path.clone();
    let device_icon = get_device_icon(&device);
    let device_tooltip = get_device_icon_tooltip(&device);
    let paired = device.paired;

    // Pre-compute colors for use in closures
    let accent_selection = theme.colors.element_selected;
    let interactive_hover = theme.colors.element_hover;
    let accent_primary = theme.colors.accent;
    let text_muted = theme.colors.text_muted;
    let text_primary = theme.colors.text;
    let status_success = theme.colors.status.success;

    let on_connect_click = on_connect.clone();
    let on_pair_click = on_pair.clone();

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "bt-device-{}",
            index
        ))))
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .w_full()
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
        .rounded(Radius::Small.pixels())
        .cursor_pointer()
        .when(connected, |el| el.bg(accent_selection))
        .when(!connected, |el| el.hover(move |s| s.bg(interactive_hover)))
        .on_mouse_down(MouseButton::Left, {
            let path = path.clone();
            move |_, _, cx| {
                if connected {
                    return;
                }

                if paired {
                    on_connect_click(path.clone(), cx);
                } else {
                    on_pair_click(path.clone(), cx);
                }
            }
        })
        // Device icon
        .child(
            div()
                .id(format!("bt-device-icon-{}", index))
                .child(
                    Icon::new(device_icon)
                        .size(IconSize::XSmall)
                        .color(Color::Custom(if connected {
                            accent_primary
                        } else {
                            text_muted
                        })),
                )
                .tooltip(control_center_tooltip(device_tooltip)),
        )
        // Device name
        .child(
            div()
                .flex_1()
                .text_size(TextSize::Small.rems())
                .text_color(text_primary)
                .overflow_hidden()
                .child(name),
        )
        .when(paired && !connected, |el| {
            el.child(
                div()
                    .id(format!("bt-paired-{}", index))
                    .child(
                        Icon::new(icons::CHECK)
                            .size(IconSize::XSmall)
                            .color(Color::Custom(status_success)),
                    )
                    .tooltip(control_center_tooltip("Paired")),
            )
        })
        // Battery level (if available)
        .when_some(battery, |el, level| {
            el.child(render_battery_indicator(index, level, cx))
        })
        .child(render_device_actions(
            index,
            connected,
            paired,
            path,
            interactive_hover,
            text_muted,
            status_success,
            on_connect,
            on_disconnect,
            on_pair,
            on_remove,
            cx,
        ))
}

#[allow(clippy::too_many_arguments)]
fn render_device_actions(
    index: usize,
    connected: bool,
    paired: bool,
    path: OwnedObjectPath,
    interactive_hover: gpui::Hsla,
    text_muted: gpui::Hsla,
    status_success: gpui::Hsla,
    on_connect: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_disconnect: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_pair: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    on_remove: impl Fn(OwnedObjectPath, &mut App) + Clone + 'static,
    _cx: &App,
) -> impl IntoElement {
    let action_button = |id: String,
                         icon: IconName,
                         color: gpui::Hsla,
                         tooltip: &'static str,
                         on_click: Box<dyn Fn(&mut App) + 'static>| {
        div()
            .id(ElementId::Name(SharedString::from(id)))
            .w(px(22.))
            .h(px(22.))
            .rounded(Radius::Small.pixels())
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(move |s| s.bg(interactive_hover))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_click(cx);
            })
            .child(
                Icon::new(icon)
                    .size(IconSize::XSmall)
                    .color(Color::Custom(color)),
            )
            .tooltip(control_center_tooltip(tooltip))
    };

    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .when(!paired, |el| {
            let path = path.clone();
            let on_pair = on_pair.clone();
            el.child(action_button(
                format!("bt-pair-{}", index),
                IconName::Plus,
                text_muted,
                "Pair",
                Box::new(move |cx| {
                    on_pair(path.clone(), cx);
                }),
            ))
        })
        .when(paired && !connected, |el| {
            let path = path.clone();
            let on_connect = on_connect.clone();
            el.child(action_button(
                format!("bt-connect-{}", index),
                icons::CHEVRON_RIGHT,
                text_muted,
                "Connect",
                Box::new(move |cx| {
                    on_connect(path.clone(), cx);
                }),
            ))
        })
        .when(connected, |el| {
            let path = path.clone();
            let on_disconnect = on_disconnect.clone();
            el.child(action_button(
                format!("bt-disconnect-{}", index),
                icons::CLOSE,
                status_success,
                "Disconnect",
                Box::new(move |cx| {
                    on_disconnect(path.clone(), cx);
                }),
            ))
        })
        .when(paired, |el| {
            let path = path.clone();
            let on_remove = on_remove.clone();
            el.child(action_button(
                format!("bt-remove-{}", index),
                icons::TRASH,
                text_muted,
                "Remove device",
                Box::new(move |cx| {
                    on_remove(path.clone(), cx);
                }),
            ))
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
///
/// Mice and gamepads have no counterpart in the icon set, so they fall
/// through to the generic Bluetooth icon - the tooltip still names them.
fn get_device_icon(device: &BluetoothDevice) -> IconName {
    // Check device class/type from icon or name hints
    let name_lower = device.name.to_lowercase();

    if name_lower.contains("airpod")
        || name_lower.contains("headphone")
        || name_lower.contains("buds")
    {
        IconName::Headphones
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
    } else if device.connected {
        icons::BLUETOOTH_CONNECTED
    } else {
        icons::BLUETOOTH
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

    let interactive_default = theme.colors.element_background;
    let interactive_hover = theme.colors.element_hover;
    let interactive_toggle_on = theme.colors.accent;
    let interactive_toggle_on_hover = theme.colors.accent;
    let bg_primary = theme.colors.background;
    let text_muted = theme.colors.text_muted;

    let bg_color = if discovering {
        interactive_toggle_on
    } else {
        interactive_default
    };
    let hover_color = if discovering {
        interactive_toggle_on_hover
    } else {
        interactive_hover
    };
    let icon_color = if discovering { bg_primary } else { text_muted };

    div()
        .id("bt-scan")
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(Radius::Small.pixels())
        .cursor_pointer()
        .bg(bg_color)
        .hover(move |s| s.bg(hover_color))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            let s = services.clone();
            cx.spawn(async move |_| {
                let _ = if discovering {
                    s.dispatch(BluetoothCommand::StopDiscovery).await
                } else {
                    s.dispatch(BluetoothCommand::StartDiscovery).await
                };
            })
            .detach();
        })
        .child(
            Icon::new(icons::REFRESH)
                .size(IconSize::XSmall)
                .color(Color::Custom(icon_color)),
        )
}
