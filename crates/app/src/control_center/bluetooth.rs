use gpui::{ClickEvent, MouseButton, div, prelude::*, px, rems};
use services::{BluetoothCommand, BluetoothDevice, BluetoothState, Services};
use ui::{IconButton, IconButtonShape, IconName, prelude::*};
use zbus::zvariant::OwnedObjectPath;

use super::icons;

pub fn bluetooth_toggle(
    services: &Services,
    expanded: bool,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let bt = services.bluetooth.get();
    let bt_enabled = bt.state == BluetoothState::Active;
    let connected_count = bt.devices.iter().filter(|d| d.connected).count();

    let toggle_expand = cx.listener(
        move |this: &mut super::ControlCenter, _evt: &ClickEvent, _w, cx| {
            this.bt_expanded = !this.bt_expanded;
            if this.bt_expanded {
                this.wifi_expanded = false;
            }
            cx.notify();
        },
    );

    let toggle_bt = cx.listener({
        let services = services.clone();
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services.bluetooth.dispatch(BluetoothCommand::Toggle).await;
            })
            .detach();
        }
    });

    let scan = cx.listener({
        let services = services.clone();
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services
                    .bluetooth
                    .dispatch(BluetoothCommand::StartDiscovery)
                    .await;
            })
            .detach();
        }
    });

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(6.0))
        .p(px(6.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div()
                .id("bt-enable-toggle")
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(3.0))
                .py(px(2.0))
                .rounded(px(6.0))
                .hover(move |s| s.bg(colors.element_hover))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, toggle_bt)
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded_full()
                        .bg(if bt_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        }),
                )
                .child(
                    div()
                        .text_size(rems(0.95))
                        .text_color(if bt_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        })
                        .child(icons::BLUETOOTH),
                )
                .when(connected_count > 0, |el| {
                    let label = if connected_count == 1 {
                        "1 Device".to_string()
                    } else {
                        format!("{connected_count} Devices")
                    };
                    el.child(
                        div()
                            .text_size(rems(0.82))
                            .text_color(colors.text)
                            .truncate()
                            .max_w(px(120.0))
                            .child(label),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(
                    IconButton::new(
                        "bt-toggle",
                        if expanded {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        },
                    )
                    .shape(IconButtonShape::Square)
                    .on_click(toggle_expand),
                )
                .child({
                    let mut btn = div()
                        .id("bt-scan")
                        .px(px(6.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .text_size(rems(0.82))
                        .text_color(if bt_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        })
                        .hover(move |s| s.bg(colors.element_hover))
                        .cursor_pointer()
                        .child(super::icons::REFRESH);
                    if bt_enabled {
                        btn = btn.on_mouse_down(MouseButton::Left, scan);
                    } else {
                        btn = btn.opacity(0.4);
                    }
                    btn
                }),
        )
}

pub fn bluetooth_list_panel(
    services: &Services,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let bt = services.bluetooth.get();

    let mut devices: Vec<BluetoothDevice> = bt.devices.clone();
    devices.sort_by(|a, b| match (a.connected, b.connected) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let list = if bt.state == BluetoothState::Active {
        if devices.is_empty() {
            let colors = cx.theme().colors();
            div()
                .py(px(8.0))
                .text_size(rems(0.82))
                .text_color(colors.text_muted)
                .child("No paired devices")
        } else {
            let mut rows = Vec::new();
            for d in devices.into_iter().take(12) {
                rows.push(device_row(d, services, cx).into_any_element());
            }
            div().flex().flex_col().gap(px(4.0)).children(rows)
        }
    } else {
        let colors = cx.theme().colors();
        div()
            .py(px(8.0))
            .text_size(rems(0.82))
            .text_color(colors.text_muted)
            .child("Bluetooth is off")
    };

    let colors = cx.theme().colors();

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .p(px(6.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div()
                .id("bt-list")
                .w_full()
                .max_h(px(200.0))
                .overflow_y_scroll()
                .child(list),
        )
}

fn device_row(
    device: BluetoothDevice,
    services: &Services,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let status = cx.theme().status();
    let name = device.name.clone();
    let connected = device.connected;
    let battery = device.battery;
    let path: OwnedObjectPath = device.path.clone();

    let on_click = cx.listener({
        let services = services.clone();
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            let services = services.clone();
            let path = path.clone();
            cx.spawn(async move |_, _| {
                let _ = if connected {
                    services
                        .bluetooth
                        .dispatch(BluetoothCommand::DisconnectDevice(path))
                        .await
                } else {
                    services
                        .bluetooth
                        .dispatch(BluetoothCommand::ConnectDevice(path))
                        .await
                };
            })
            .detach();
        }
    });

    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .when(!connected, |el| {
            el.hover(move |s| s.bg(colors.element_hover))
        })
        .when(connected, |el| el.bg(colors.element_active))
        .on_mouse_down(MouseButton::Left, on_click)
        .child(
            div()
                .text_size(rems(0.9))
                .text_color(if connected {
                    colors.text_accent
                } else {
                    colors.text_muted
                })
                .child(if connected {
                    icons::BLUETOOTH_CONNECTED
                } else {
                    icons::BLUETOOTH
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(rems(0.9))
                        .text_color(colors.text)
                        .truncate()
                        .child(name),
                )
                .child(
                    div()
                        .text_size(rems(0.8))
                        .text_color(if connected {
                            status.success
                        } else {
                            colors.text_muted
                        })
                        .child(if connected { "Connected" } else { "Paired" }),
                ),
        )
        .when_some(battery, |el, level| {
            el.child(
                div()
                    .text_size(rems(0.82))
                    .text_color(if level <= 20 {
                        status.error
                    } else if level <= 40 {
                        status.warning
                    } else {
                        colors.text_muted
                    })
                    .child(format!("{}%", level)),
            )
        })
}
