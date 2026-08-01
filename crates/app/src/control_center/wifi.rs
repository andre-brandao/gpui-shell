//! WiFi section for the Control Center.
//!
//! Displays available networks with signal strength and connection status.
//! Supports connecting to open and protected networks with password input.

use gpui::{App, ElementId, MouseButton, SharedString, div, prelude::*, px};
use services::{AccessPoint, NetworkCommand};
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, InputBuffer, Radius, Spacing, TextSize,
    render_masked_input_line,
};

use crate::state::AppState;

use super::tooltip::control_center_tooltip;
use crate::icons;

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
    let list_bg = theme.colors.background;
    let list_border = theme.colors.border_variant;

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
    aps.sort_by(|a, b| {
        let a_connected = connected_name.as_ref() == Some(&a.ssid);
        let b_connected = connected_name.as_ref() == Some(&b.ssid);
        match (a_connected, b_connected) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.strength.cmp(&a.strength),
        }
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
                .gap(Spacing::Medium.pixels())
                .child(
                    Icon::new(IconName::Wifi)
                        .size(IconSize::XSmall)
                        .color(Color::Default),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(TextSize::Small.rems())
                        .text_color(theme.colors.text)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("WiFi"),
                )
                .when_some(connected_name.clone(), |el, name| {
                    el.child(
                        div()
                            .text_size(TextSize::XSmall.rems())
                            .text_color(theme.colors.text)
                            .child(format!("- {}", name)),
                    )
                })
                .child(render_refresh_button(cx)),
        )
        .when(!wifi_enabled, |el| {
            el.child(
                div()
                    .py(Spacing::Large.pixels())
                    .text_size(TextSize::Small.rems())
                    .text_color(theme.colors.text_muted)
                    .text_center()
                    .child("WiFi is off"),
            )
        })
        .when(wifi_enabled && aps.is_empty(), |el| {
            el.child(
                div()
                    .py(Spacing::Large.pixels())
                    .text_size(TextSize::Small.rems())
                    .text_color(theme.colors.text_muted)
                    .text_center()
                    .child("No networks found"),
            )
        })
        .when(wifi_enabled && !aps.is_empty(), |el| {
            el.child(
                div()
                    .id("wifi-networks-list")
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
                    .children(aps.into_iter().enumerate().map(|(idx, ap)| {
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
                    })),
            )
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
    let accent_selection = theme.colors.element_selected;
    let interactive_hover = theme.colors.element_hover;
    let accent_primary = theme.colors.accent;
    let text_muted = theme.colors.text_muted;
    let text_primary = theme.colors.text;
    let status_success = theme.colors.status.success;

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "wifi-{}",
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
                .child(
                    Icon::new(signal_icon)
                        .size(IconSize::XSmall)
                        .color(Color::Custom(if connected {
                            accent_primary
                        } else {
                            text_muted
                        })),
                )
                .tooltip(control_center_tooltip(format!(
                    "Signal strength: {}%",
                    strength
                ))),
        )
        // Network name
        .child(
            div()
                .flex_1()
                .text_size(TextSize::Small.rems())
                .text_color(text_primary)
                .overflow_hidden()
                .child(ssid.to_string()),
        )
        .when(known && !connected, |el| {
            el.child(
                div()
                    .id(format!("wifi-known-{}", index))
                    .child(
                        Icon::new(IconName::Check)
                            .size(IconSize::XSmall)
                            .color(Color::Custom(status_success)),
                    )
                    .tooltip(control_center_tooltip("Saved network")),
            )
        })
        // Lock icon for secured networks (green if known/saved)
        .when(secured, move |el| {
            el.child(
                div()
                    .id(format!("wifi-lock-{}", index))
                    .child(
                        Icon::new(IconName::Lock)
                            .size(IconSize::XSmall)
                            .color(Color::Custom(if known {
                                status_success
                            } else {
                                text_muted
                            })),
                    )
                    .tooltip(control_center_tooltip(lock_tooltip)),
            )
        })
        .when(!connected, |el| {
            el.child(
                div()
                    .id(format!("wifi-connect-{}", index))
                    .child(
                        Icon::new(IconName::ChevronRight)
                            .size(IconSize::XSmall)
                            .color(Color::Muted),
                    )
                    .tooltip(control_center_tooltip("Connect")),
            )
        })
        .when(connected, move |el| {
            let disconnect_ssid = disconnect_ssid.clone();
            el.child(
                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "wifi-disconnect-{}",
                        index
                    ))))
                    .w(px(22.))
                    .h(px(22.))
                    .rounded(Radius::Small.pixels())
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(interactive_hover))
                    .tooltip(control_center_tooltip("Disconnect"))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        if let Some(ssid) = disconnect_ssid.clone() {
                            on_disconnect(ssid, cx);
                        }
                    })
                    .child(
                        Icon::new(IconName::Close)
                            .size(IconSize::XSmall)
                            .color(Color::Custom(status_success)),
                    ),
            )
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
    let bg_tertiary = theme.colors.elevated_surface_background;
    let bg_primary = theme.colors.background;
    let accent_primary = theme.colors.accent;
    let accent_hover = theme.colors.accent;
    let text_primary = theme.colors.text;
    let text_muted = theme.colors.text_muted;
    let status_error = theme.colors.status.error;

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "wifi-password-{}",
            index
        ))))
        .flex()
        .flex_col()
        .gap(Spacing::XSmall.pixels())
        .w_full()
        .px(Spacing::Medium.pixels())
        .py(Spacing::Medium.pixels())
        .bg(bg_tertiary)
        .rounded(Radius::Small.pixels())
        // Network name header
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(
                    Icon::new(IconName::Lock)
                        .size(IconSize::XSmall)
                        .color(Color::Accent),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(TextSize::Small.rems())
                        .text_color(text_primary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(ssid.to_string()),
                )
                .child(
                    div()
                        .id(format!("wifi-cancel-{}", index))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            on_cancel(cx);
                        })
                        .child(
                            Icon::new(IconName::Close)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        ),
                ),
        )
        // Password input with keyboard support
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(
                    div()
                        .flex_1()
                        .px(Spacing::Medium.pixels())
                        .py(Spacing::XSmall.pixels())
                        .bg(bg_primary)
                        .rounded(Radius::Small.pixels())
                        .border_1()
                        .border_color(accent_primary)
                        .child(
                            div()
                                .text_size(TextSize::Small.rems())
                                .text_color(text_primary)
                                .child(password_line),
                        ),
                )
                .child(
                    div()
                        .id(format!("wifi-connect-{}", index))
                        .px(Spacing::Large.pixels())
                        .py(Spacing::XSmall.pixels())
                        .bg(accent_primary)
                        .rounded(Radius::Small.pixels())
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
                                .text_size(TextSize::Small.rems())
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
                .text_size(TextSize::XSmall.rems())
                .text_color(text_muted)
                .child("Press Enter to connect, Escape to cancel"),
        )
        // Error message
        .when_some(error, |el, err| {
            el.child(
                div()
                    .text_size(TextSize::XSmall.rems())
                    .text_color(status_error)
                    .child(err.to_string()),
            )
        })
}

/// Render a refresh button for rescanning networks
pub fn render_refresh_button(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let services = AppState::network(cx).clone();

    // Pre-compute colors for closures
    let interactive_default = theme.colors.element_background;
    let interactive_hover = theme.colors.element_hover;

    div()
        .id("wifi-refresh")
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(Radius::Small.pixels())
        .cursor_pointer()
        .bg(interactive_default)
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            let s = services.clone();
            cx.spawn(async move |_| {
                let _ = s.dispatch(NetworkCommand::RequestScan).await;
            })
            .detach();
        })
        .child(
            Icon::new(IconName::Refresh)
                .size(IconSize::XSmall)
                .color(Color::Muted),
        )
}
