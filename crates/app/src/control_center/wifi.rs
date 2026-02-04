//! WiFi section for the Control Center.
//!
//! Displays available networks with signal strength and connection status.
//! Supports connecting to open and protected networks with password input.

use gpui::{App, ElementId, MouseButton, SharedString, div, prelude::*, px};
use services::{AccessPoint, NetworkCommand, Services};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use super::icons;

/// State for WiFi password input
#[derive(Debug, Clone, Default)]
pub struct WifiPasswordState {
    /// The SSID we're trying to connect to
    pub ssid: Option<String>,
    /// The current password input
    pub password: String,
    /// Whether we're currently connecting
    pub connecting: bool,
    /// Error message if connection failed
    pub error: Option<String>,
}

impl WifiPasswordState {
    /// Start password entry for a network
    pub fn start_for(&mut self, ssid: String) {
        self.ssid = Some(ssid);
        self.password.clear();
        self.connecting = false;
        self.error = None;
    }

    /// Clear the password state
    pub fn clear(&mut self) {
        self.ssid = None;
        self.password.clear();
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
    services: &Services,
    password_state: &WifiPasswordState,
    on_connect: impl Fn(String, Option<String>, &mut App) + Clone + 'static,
    on_password_change: impl Fn(String, &mut App) + Clone + 'static,
    on_cancel_password: impl Fn(&mut App) + Clone + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let network = services.network.get();

    // Get current connection name
    let connected_name: Option<String> = network.active_connections.iter().find_map(|c| {
        if let services::ActiveConnectionInfo::WiFi { name, .. } = c {
            Some(name.clone())
        } else {
            None
        }
    });

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
        .gap(px(spacing::XS))
        .p(px(spacing::SM))
        .bg(theme.bg.secondary)
        .rounded(px(radius::MD))
        .border_1()
        .border_color(theme.border.subtle)
        .child(
            // Section header
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .pb(px(spacing::XS))
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(theme.text.muted)
                        .child(icons::WIFI),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(font_size::SM))
                        .text_color(theme.text.secondary)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("Networks"),
                )
                .child(render_refresh_button(services, cx)),
        )
        .when(aps.is_empty(), |el| {
            el.child(
                div()
                    .py(px(spacing::MD))
                    .text_size(px(font_size::SM))
                    .text_color(theme.text.muted)
                    .text_center()
                    .child("No networks found"),
            )
        })
        .when(!aps.is_empty(), |el| {
            el.child(
                div()
                    .id("wifi-networks-list")
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .max_h(px(200.))
                    .overflow_y_scroll()
                    .children(aps.into_iter().enumerate().map(|(idx, ap)| {
                        let is_connected = connected_name.as_ref() == Some(&ap.ssid);
                        let is_entering_password = password_state.is_entering_for(&ap.ssid);
                        let ssid = ap.ssid.clone();
                        let ssid_for_display = ssid.clone();
                        let ssid_for_callback = ssid.clone();
                        let is_secured = !ap.public;
                        let is_known = ap.known;
                        let on_connect = on_connect.clone();
                        let on_password_change = on_password_change.clone();
                        let on_cancel = on_cancel_password.clone();
                        let current_password = password_state.password.clone();
                        let is_connecting = password_state.connecting;

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
                                on_password_change.clone(),
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
                                cx,
                            )
                            .into_any_element()
                        }
                    })),
            )
        })
}

/// Render a single network item in the list
fn render_network_item(
    index: usize,
    ssid: &str,
    strength: u8,
    secured: bool,
    known: bool,
    connected: bool,
    on_click: impl Fn(&mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let signal_icon = icons::wifi_signal_icon(strength);

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
                .text_size(px(icon_size::SM))
                .text_color(if connected {
                    accent_primary
                } else {
                    text_muted
                })
                .child(signal_icon),
        )
        // Network name
        .child(
            div()
                .flex_1()
                .text_size(px(font_size::SM))
                .text_color(text_primary)
                .overflow_hidden()
                .child(ssid.to_string()),
        )
        // Lock icon for secured networks (green if known/saved)
        .when(secured, |el| {
            el.child(
                div()
                    .text_size(px(icon_size::SM))
                    .text_color(if known { status_success } else { text_muted })
                    .child(icons::LOCK),
            )
        })
        // Connected checkmark
        .when(connected, |el| {
            el.child(
                div()
                    .text_size(px(icon_size::SM))
                    .text_color(status_success)
                    .child(icons::CHECK),
            )
        })
}

/// Render password input row for a network
fn render_password_input(
    index: usize,
    ssid: &str,
    current_password: &str,
    connecting: bool,
    error: Option<&str>,
    on_submit: impl Fn(String, &mut App) + 'static,
    _on_change: impl Fn(String, &mut App) + 'static,
    on_cancel: impl Fn(&mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let password = current_password.to_string();
    let password_for_submit = password.clone();

    // Pre-compute colors for closures
    let bg_tertiary = theme.bg.tertiary;
    let bg_primary = theme.bg.primary;
    let accent_primary = theme.accent.primary;
    let accent_hover = theme.accent.hover;
    let text_primary = theme.text.primary;
    let text_muted = theme.text.muted;
    let text_placeholder = theme.text.placeholder;
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
                        .text_size(px(font_size::SM))
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
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .text_color(if password.is_empty() {
                                            text_placeholder
                                        } else {
                                            text_primary
                                        })
                                        .child(if password.is_empty() {
                                            "Type password...".to_string()
                                        } else {
                                            "•".repeat(password.len())
                                        }),
                                )
                                // Blinking cursor indicator
                                .child(div().w(px(1.)).h(px(14.)).bg(text_primary).ml(px(1.))),
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
                                .text_size(px(font_size::SM))
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
                .text_size(px(font_size::XS))
                .text_color(text_muted)
                .child("Press Enter to connect, Escape to cancel"),
        )
        // Error message
        .when_some(error, |el, err| {
            el.child(
                div()
                    .text_size(px(font_size::XS))
                    .text_color(status_error)
                    .child(err.to_string()),
            )
        })
}

/// Render a refresh button for rescanning networks
pub fn render_refresh_button(services: &Services, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let services = services.clone();

    // Pre-compute colors for closures
    let interactive_default = theme.interactive.default;
    let interactive_hover = theme.interactive.hover;
    let text_muted = theme.text.muted;

    div()
        .id("wifi-refresh")
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .bg(interactive_default)
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            let s = services.clone();
            cx.spawn(async move |_| {
                let _ = s.network.dispatch(NetworkCommand::RequestScan).await;
            })
            .detach();
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(text_muted)
                .child(icons::REFRESH),
        )
}
