use gpui::StatefulInteractiveElement;
use gpui::{ClickEvent, MouseButton, div, prelude::*, px, rems};
use services::{AccessPoint, ActiveConnectionInfo, NetworkCommand, Services};
use ui::{IconButton, IconButtonShape, IconName, prelude::*};
use zbus::zvariant::OwnedObjectPath;

use super::icons;

pub fn wifi_toggle(
    services: &Services,
    expanded: bool,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let network = services.network.get();
    let wifi_enabled = network.wifi_enabled;
    let connected_ssid = network.active_connections.iter().find_map(|c| {
        if let ActiveConnectionInfo::WiFi { name, .. } = c {
            Some(name.clone())
        } else {
            None
        }
    });

    let toggle_expand = cx.listener(
        move |this: &mut super::ControlCenter, _evt: &ClickEvent, _w, cx| {
            this.wifi_expanded = !this.wifi_expanded;
            if this.wifi_expanded {
                this.bt_expanded = false;
            }
            cx.notify();
        },
    );

    let toggle_wifi = cx.listener({
        let services = services.clone();
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services.network.dispatch(NetworkCommand::ToggleWifi).await;
            })
            .detach();
        }
    });

    let scan = cx.listener({
        let services = services.clone();
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services.network.dispatch(NetworkCommand::RequestScan).await;
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
                .id("wifi-enable-toggle")
                .flex()
                .items_center()
                .gap(px(6.0))
                .min_w(px(0.0))
                .px(px(3.0))
                .py(px(2.0))
                .rounded(px(6.0))
                .hover(move |s| s.bg(colors.element_hover))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, toggle_wifi)
                .child(
                    div()
                        .text_size(rems(0.95))
                        .text_color(if wifi_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        })
                        .child(icons::WIFI),
                )
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded_full()
                        .bg(if wifi_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        }),
                )
                .when_some(connected_ssid, |el, ssid| {
                    el.child(
                        div()
                            .text_size(rems(0.82))
                            .text_color(colors.text)
                            .truncate()
                            .max_w(px(120.0))
                            .child(ssid),
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
                        "wifi-toggle",
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
                        .id("wifi-scan")
                        .px(px(6.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .text_size(rems(0.82))
                        .text_color(if wifi_enabled {
                            colors.text_accent
                        } else {
                            colors.text_muted
                        })
                        .hover(move |s| s.bg(colors.element_hover))
                        .cursor_pointer()
                        .child(super::icons::REFRESH);
                    if wifi_enabled {
                        btn = btn.on_mouse_down(MouseButton::Left, scan);
                    } else {
                        btn = btn.opacity(0.4);
                    }
                    btn
                }),
        )
}

pub fn wifi_list_panel(
    services: &Services,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let network = services.network.get();
    let wifi_enabled = network.wifi_enabled;
    let connected_ssid = network.active_connections.iter().find_map(|c| {
        if let ActiveConnectionInfo::WiFi { name, .. } = c {
            Some(name.clone())
        } else {
            None
        }
    });

    let mut aps: Vec<AccessPoint> = network.wireless_access_points.clone();
    aps.sort_by(|a, b| {
        let a_connected = connected_ssid.as_ref() == Some(&a.ssid);
        let b_connected = connected_ssid.as_ref() == Some(&b.ssid);
        match (a_connected, b_connected) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.strength.cmp(&a.strength),
        }
    });

    let list = if wifi_enabled && !aps.is_empty() {
        let mut rows = Vec::new();
        for ap in aps.into_iter().take(12) {
            let is_connected = connected_ssid.as_ref() == Some(&ap.ssid);
            rows.push(network_row(ap, is_connected, services, cx).into_any_element());
        }
        div().flex().flex_col().gap(px(4.0)).children(rows)
    } else {
        let colors = cx.theme().colors();
        div()
            .py(px(8.0))
            .text_size(rems(0.82))
            .text_color(colors.text_muted)
            .child(if wifi_enabled {
                "No networks found"
            } else {
                "Wifi is off"
            })
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
                .id("wifi-list")
                .w_full()
                .max_h(px(200.0))
                .overflow_y_scroll()
                .child(list),
        )
}

fn network_row(
    ap: AccessPoint,
    is_connected: bool,
    services: &Services,
    cx: &mut gpui::Context<'_, super::ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let status = cx.theme().status();
    let ssid = ap.ssid.clone();
    let signal_icon = icons::wifi_signal_icon(ap.strength);
    let lock = !ap.public;

    let on_connect = cx.listener({
        let services = services.clone();
        let ap_path: OwnedObjectPath = ap.path.clone().into();
        let device_path: OwnedObjectPath = ap.device_path.clone().into();
        let needs_password = lock && !ap.known;
        move |_, _event: &gpui::MouseDownEvent, _window, cx| {
            if needs_password || is_connected {
                return;
            }
            let services = services.clone();
            let ap_path = ap_path.clone();
            let device_path = device_path.clone();
            cx.spawn(async move |_, _| {
                let _ = services
                    .network
                    .dispatch(NetworkCommand::ConnectToAccessPoint {
                        device_path,
                        ap_path,
                        password: None,
                    })
                    .await;
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
        .when(!is_connected, |el| {
            el.hover(move |s| s.bg(colors.element_hover))
        })
        .when(is_connected, |el| el.bg(colors.element_active))
        .on_mouse_down(MouseButton::Left, on_connect)
        .child(
            div()
                .text_size(rems(0.9))
                .text_color(if is_connected {
                    colors.text_accent
                } else {
                    colors.text_muted
                })
                .child(signal_icon),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(rems(0.9))
                        .text_color(colors.text)
                        .truncate()
                        .child(ssid),
                )
                .when(lock, |el| {
                    el.child(
                        div()
                            .text_size(rems(0.8))
                            .text_color(colors.text_muted)
                            .child(icons::WIFI_LOCK),
                    )
                }),
        )
        .child({
            let text = if is_connected {
                "Connected".to_string()
            } else if lock && !ap.known {
                "Password".to_string()
            } else if ap.working {
                "Connecting...".to_string()
            } else {
                format!("{}%", ap.strength)
            };

            div()
                .text_size(rems(0.82))
                .text_color(if is_connected {
                    status.success
                } else if lock && !ap.known {
                    status.warning
                } else {
                    colors.text_muted
                })
                .child(text)
        })
}
