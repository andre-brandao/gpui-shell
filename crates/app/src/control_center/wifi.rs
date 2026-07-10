//! WiFi section for the Control Center.
//!
//! Displays available networks with signal strength and connection status.
//! Supports connecting to open and protected networks with password input.

use gpui::{App, ElementId, MouseButton, SharedString, div, prelude::*, px};
use services::{AccessPoint, NetworkCommand};
use ui::{ActiveTheme, InputBuffer, icon_size, radius, render_masked_input_line, spacing};

use crate::state::AppState;

use super::section::{
    row_action_button, section_empty_state, section_header, section_list, section_scan_button,
    sort_ranked,
};
use super::{icons, tooltip::control_center_tooltip};

/// State for WiFi password input
#[derive(Debug, Clone, Default)]
pub struct WifiPasswordState {
    /// The SSID we're trying to connect to
    pub ssid: Option<String>,
    /// The current password input
    pub input: InputBuffer,
    /// Whether we're currently connecting
    pub connecting: bool,
    /// Error message if connection failed
    pub error: Option<String>,
}

impl WifiPasswordState {
    /// Start password entry for a network
    pub fn start_for(&mut self, ssid: String) {
        self.ssid = Some(ssid);
        self.input.clear();
        self.connecting = false;
        self.error = None;
    }

    /// Clear the password state
    pub fn clear(&mut self) {
        self.ssid = None;
        self.input.clear();
        self.connecting = false;
        self.error = None;
    }

    /// Check if we're entering a password for a specific SSID
    pub fn is_entering_for(&self, ssid: &str) -> bool {
        self.ssid.as_deref() == Some(ssid)
    }
}

/// Render the WiFi section (network list)
pub fn render_wifi_section(
    password_state: &WifiPasswordState,
    on_connect: impl Fn(String, Option<String>, &mut App) + Clone + 'static,
    on_disconnect: impl Fn(String, &mut App) + Clone + 'static,
    on_cancel_password: impl Fn(&mut App) + Clone + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let network = AppState::network(cx).get();

    // Get current connected WiFi SSID
    let connected_name = network.active_connections.iter().find_map(|c| {
        if let services::ActiveConnectionInfo::WiFi { name, .. } = c {
            Some(name.clone())
        } else {
            None
        }
    });
    let wifi_enabled = network.wifi_enabled;

    // Sort access points: connected first, then by signal strength
    let mut aps: Vec<AccessPoint> = network.wireless_access_points.clone();
    let connected_for_sort = connected_name.clone();
    sort_ranked(
        &mut aps,
        move |ap| u8::from(connected_for_sort.as_ref() != Some(&ap.ssid)),
        |a, b| b.strength.cmp(&a.strength),
    );

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(spacing::SM))
        .child(
            section_header(icons::WIFI, "WiFi", cx)
                .when_some(connected_name.clone(), |el, name| {
                    el.child(
                        div()
                            .text_size(theme.font_sizes.xs)
                            .text_color(theme.text.muted)
                            .child(format!("- {}", name)),
                    )
                })
                .child(render_refresh_button(cx)),
        )
        .when(!wifi_enabled, |el| {
            el.child(section_empty_state("WiFi is off", cx))
        })
        .when(wifi_enabled && aps.is_empty(), |el| {
            el.child(section_empty_state("No networks found", cx))
        })
        .when(wifi_enabled && !aps.is_empty(), |el| {
            el.child(section_list("wifi-networks-list", cx).children(
                aps.into_iter().enumerate().map(|(idx, ap)| {
                    let is_connected = connected_name.as_ref() == Some(&ap.ssid);
                    let is_entering_password = password_state.is_entering_for(&ap.ssid);
                    let ssid = ap.ssid.clone();
                    let ssid_for_display = ssid.clone();
                    let ssid_for_callback = ssid.clone();
                    let is_secured = !ap.public;
                    let is_known = ap.known;
                    let on_connect = on_connect.clone();
                    let on_disconnect = on_disconnect.clone();
                    let on_cancel = on_cancel_password.clone();
                    let current_password = password_state.input.clone();
                    let is_connecting = password_state.connecting;
                    let disconnect_ssid = connected_name.clone();

                    if is_entering_password {
                        let ssid_submit = ssid.clone();
                        render_password_input(
                            idx,
                            &ssid_for_display,
                            &current_password,
                            is_connecting,
                            password_state.error.as_deref(),
                            move |password, cx| {
                                let ssid = ssid_submit.clone();
                                on_connect(ssid, Some(password), cx);
                            },
                            on_cancel,
                            cx,
                        )
                        .into_any_element()
                    } else {
                        render_network_item(
                            idx,
                            &ssid_for_display,
                            ap.strength,
                            is_secured,
                            is_known,
                            is_connected,
                            disconnect_ssid.clone(),
                            move |cx| {
                                if is_connected {
                                    // Already connected, do nothing or disconnect
                                    return;
                                }

                                let ssid = ssid_for_callback.clone();

                                if is_secured && !is_known {
                                    // Need password - this will be handled by the parent
                                    on_connect(ssid, None, cx);
                                } else {
                                    // Open network or known network - connect directly
                                    // For known networks, NM will use saved credentials
                                    on_connect(ssid, Some(String::new()), cx);
                                }
                            },
                            move |path, cx| {
                                on_disconnect(path, cx);
                            },
                            cx,
                        )
                        .into_any_element()
                    }
                }),
            ))
        })
}

/// Render a single network item in the list
#[allow(clippy::too_many_arguments)]
fn render_network_item(
    index: usize,
    ssid: &str,
    strength: u8,
    secured: bool,
    known: bool,
    connected: bool,
    disconnect_ssid: Option<String>,
    on_click: impl Fn(&mut App) + 'static,
    on_disconnect: impl Fn(String, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let signal_icon = icons::wifi_signal_icon(strength);
    let lock_tooltip = if known {
        "Secured (saved)"
    } else {
        "Secured (password required)"
    };

    // Pre-compute colors for closures
    let accent_selection = theme.accent.selection;
    let interactive_hover = theme.interactive.hover;
    let accent_primary = theme.accent.primary;
    let text_muted = theme.text.muted;
    let text_primary = theme.text.primary;
    let status_success = theme.status.success;

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "wifi-{}",
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
        .when(connected, move |el| el.bg(accent_selection))
        .when(!connected, move |el| {
            el.hover(move |s| s.bg(interactive_hover))
        })
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        // Signal strength icon
        .child(
            div()
                .id(format!("wifi-signal-{}", index))
                .text_size(px(icon_size::SM))
                .text_color(if connected {
                    accent_primary
                } else {
                    text_muted
                })
                .child(signal_icon)
                .tooltip(control_center_tooltip(format!(
                    "Signal strength: {}%",
                    strength
                ))),
        )
        // Network name
        .child(
            div()
                .flex_1()
                .text_size(theme.font_sizes.sm)
                .text_color(text_primary)
                .overflow_hidden()
                .child(ssid.to_string()),
        )
        .when(known && !connected, |el| {
            el.child(
                div()
                    .id(format!("wifi-known-{}", index))
                    .text_size(px(icon_size::SM))
                    .text_color(status_success)
                    .child(icons::CHECK)
                    .tooltip(control_center_tooltip("Saved network")),
            )
        })
        // Lock icon for secured networks (green if known/saved)
        .when(secured, move |el| {
            el.child(
                div()
                    .id(format!("wifi-lock-{}", index))
                    .text_size(px(icon_size::SM))
                    .text_color(if known { status_success } else { text_muted })
                    .child(icons::LOCK)
                    .tooltip(control_center_tooltip(lock_tooltip)),
            )
        })
        .when(!connected, |el| {
            el.child(
                div()
                    .id(format!("wifi-connect-{}", index))
                    .text_size(px(icon_size::SM))
                    .text_color(text_muted)
                    .child(icons::CHEVRON_RIGHT)
                    .tooltip(control_center_tooltip("Connect")),
            )
        })
        .when(connected, move |el| {
            let disconnect_ssid = disconnect_ssid.clone();
            el.child(row_action_button(
                format!("wifi-disconnect-{}", index),
                icons::CLOSE,
                status_success,
                interactive_hover,
                "Disconnect",
                move |cx| {
                    if let Some(ssid) = disconnect_ssid.clone() {
                        on_disconnect(ssid, cx);
                    }
                },
            ))
        })
}

/// Render password input row for a network
#[allow(clippy::too_many_arguments)]
fn render_password_input(
    index: usize,
    ssid: &str,
    current_password: &InputBuffer,
    connecting: bool,
    error: Option<&str>,
    on_submit: impl Fn(String, &mut App) + 'static,
    on_cancel: impl Fn(&mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let password_for_submit = current_password.text().to_string();
    let password_line = render_masked_input_line(current_password, "Type password...", '•', cx);

    // Pre-compute colors for closures
    let bg_tertiary = theme.bg.tertiary;
    let bg_primary = theme.bg.primary;
    let accent_primary = theme.accent.primary;
    let accent_hover = theme.accent.hover;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let status_error = theme.status.error;

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "wifi-password-{}",
            index
        ))))
        .flex()
        .flex_col()
        .gap(px(spacing::XS))
        .w_full()
        .px(px(spacing::SM))
        .py(px(spacing::SM))
        .bg(bg_tertiary)
        .rounded(px(radius::SM))
        // Network name header
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(accent_primary)
                        .child(icons::WIFI_LOCK),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(theme.font_sizes.sm)
                        .text_color(text_primary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(ssid.to_string()),
                )
                .child(
                    div()
                        .id(format!("wifi-cancel-{}", index))
                        .text_size(px(icon_size::SM))
                        .text_color(text_muted)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(text_primary))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            on_cancel(cx);
                        })
                        .child(icons::CLOSE),
                ),
        )
        // Password input with keyboard support
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(
                    div()
                        .flex_1()
                        .px(px(spacing::SM))
                        .py(px(spacing::XS))
                        .bg(bg_primary)
                        .rounded(px(radius::SM))
                        .border_1()
                        .border_color(accent_primary)
                        .child(
                            div()
                                .text_size(theme.font_sizes.sm)
                                .text_color(text_primary)
                                .child(password_line),
                        ),
                )
                .child(
                    div()
                        .id(format!("wifi-connect-{}", index))
                        .px(px(spacing::MD))
                        .py(px(spacing::XS))
                        .bg(accent_primary)
                        .rounded(px(radius::SM))
                        .cursor_pointer()
                        .hover(move |s| s.bg(accent_hover))
                        .when(connecting, |el| el.opacity(0.7))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            if !connecting {
                                on_submit(password_for_submit.clone(), cx);
                            }
                        })
                        .child(
                            div()
                                .text_size(theme.font_sizes.sm)
                                .text_color(bg_primary)
                                .child(if connecting {
                                    "Connecting..."
                                } else {
                                    "Connect"
                                }),
                        ),
                ),
        )
        // Keyboard hints
        .child(
            div()
                .text_size(theme.font_sizes.xs)
                .text_color(text_muted)
                .child("Press Enter to connect, Escape to cancel"),
        )
        // Error message
        .when_some(error, |el, err| {
            el.child(
                div()
                    .text_size(theme.font_sizes.xs)
                    .text_color(status_error)
                    .child(err.to_string()),
            )
        })
}

/// Render a refresh button for rescanning networks
fn render_refresh_button(cx: &App) -> impl IntoElement {
    let services = AppState::network(cx).clone();
    section_scan_button(
        "wifi-refresh",
        false,
        move |cx| {
            let s = services.clone();
            cx.spawn(async move |_| {
                let _ = s.dispatch(NetworkCommand::RequestScan).await;
            })
            .detach();
        },
        cx,
    )
}
